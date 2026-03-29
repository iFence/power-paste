use std::sync::{atomic::Ordering, Arc, Mutex};

use anyhow::{Context, Result};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, PhysicalPosition, PhysicalSize, Position, Size, WindowEvent,
};

#[cfg(windows)]
use webview2_com::{
    ContextMenuRequestedEventHandler, Microsoft::Web::WebView2::Win32::ICoreWebView2_11,
};
#[cfg(windows)]
use windows_core::Interface;

use crate::{
    models::{SharedState, PANEL_LABEL},
    paste_target::remember_last_target_window,
    save_settings,
};

// Toggles the panel near the cursor and remembers the previous app for later paste-back.
pub(crate) fn toggle_panel(app: &AppHandle) -> Result<()> {
    let window = app
        .get_webview_window(PANEL_LABEL)
        .context("main window not found")?;

    if window.is_visible()? {
        if window.is_focused()? {
            window.hide()?;
        } else {
            window.show()?;
            window.unminimize()?;
            window.set_focus()?;
        }
    } else {
        remember_last_target_window(app);
        let cursor = app.cursor_position()?;
        let monitor = app.monitor_from_point(cursor.x, cursor.y)?;
        let size = window.outer_size()?;

        if let Some(monitor) = monitor {
            let screen_origin = monitor.position();
            let screen_size = monitor.size();
            let margin = 16i32;

            let mut target_x = cursor.x.round() as i32 - 32;
            let mut target_y = cursor.y.round() as i32 + 18;
            let min_x = screen_origin.x + margin;
            let min_y = screen_origin.y + margin;
            let max_x = screen_origin.x + screen_size.width as i32 - size.width as i32 - margin;
            let max_y = screen_origin.y + screen_size.height as i32 - size.height as i32 - margin;

            target_x = target_x.clamp(min_x, max_x.max(min_x));
            target_y = target_y.clamp(min_y, max_y.max(min_y));

            window.set_position(Position::Physical(PhysicalPosition::new(
                target_x, target_y,
            )))?;
        }

        window.show()?;
        window.set_focus()?;
    }

    Ok(())
}

// Applies persisted window state and wires tray/webview event handlers.
pub(crate) fn configure_window(app: &AppHandle, shared: Arc<SharedState>) -> Result<()> {
    let window = app
        .get_webview_window(PANEL_LABEL)
        .context("main window not found")?;
    let window_clone = window.clone();
    let event_shared = shared.clone();
    #[cfg(windows)]
    let context_menu_enabled = shared.debug_context_menu_enabled.clone();

    if let Some(icon) = app.default_window_icon().cloned() {
        window.set_icon(icon)?;
    }

    {
        let settings = shared.settings.lock().unwrap().clone();
        shared
            .debug_context_menu_enabled
            .store(settings.debug_enabled, Ordering::Relaxed);
        crate::apply_debug_mode(&window, settings.debug_enabled)?;
        if let (Some(x), Some(y)) = (settings.window_x, settings.window_y) {
            window.set_position(Position::Physical(PhysicalPosition::new(x, y)))?;
        }
        if let (Some(width), Some(height)) = (settings.window_width, settings.window_height) {
            window.set_size(Size::Physical(PhysicalSize::new(width, height)))?;
        }
    }

    #[cfg(windows)]
    {
        let webview_result = Arc::new(Mutex::new(Ok(())));
        let webview_result_clone = webview_result.clone();
        window
            .with_webview(move |webview| {
                let result = (|| -> Result<()> {
                    let controller = webview.controller();
                    let webview = unsafe { controller.CoreWebView2() }
                        .context("failed to access CoreWebView2 controller")?;
                    let webview = webview
                        .cast::<ICoreWebView2_11>()
                        .context("failed to access ICoreWebView2_11")?;

                    let mut token = 0i64;
                    unsafe {
                        webview.add_ContextMenuRequested(
                            &ContextMenuRequestedEventHandler::create(Box::new(move |_, args| {
                                let Some(args) = args else {
                                    return Ok(());
                                };

                                if !context_menu_enabled.load(Ordering::Relaxed) {
                                    args.SetHandled(true)?;
                                }

                                Ok(())
                            })),
                            &mut token,
                        )?;
                    }

                    Ok(())
                })();
                *webview_result_clone.lock().unwrap() = result;
            })
            .context("failed to access platform webview")?;
        let webview_result_guard = webview_result.lock().unwrap();
        if let Err(error) = webview_result_guard.as_ref() {
            return Err(anyhow::anyhow!(error.to_string()));
        }
    }

    let _ = window.hide();

    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = window_clone.hide();
        }
        WindowEvent::Moved(position) => {
            let mut settings = event_shared.settings.lock().unwrap();
            settings.window_x = Some(position.x);
            settings.window_y = Some(position.y);
            let _ = save_settings(&event_shared.paths, &settings);
        }
        WindowEvent::Resized(size) => {
            let mut settings = event_shared.settings.lock().unwrap();
            settings.window_width = Some(size.width);
            settings.window_height = Some(size.height);
            let _ = save_settings(&event_shared.paths, &settings);
        }
        _ => {}
    });

    Ok(())
}

fn tray_label(locale: &str, key: &str) -> &'static str {
    if locale == "zh-CN" {
        match key {
            "show" => "显示 Power Paste",
            "devtools" => "打开 DevTools",
            "quit" => "退出",
            _ => "",
        }
    } else {
        match key {
            "show" => "Show Power Paste",
            "devtools" => "Open DevTools",
            "quit" => "Quit",
            _ => "",
        }
    }
}

// The tray mirrors the main show/quit actions so the app can stay background-resident.
pub(crate) fn build_tray(app: &AppHandle, locale: &str) -> Result<()> {
    let version_prefix = if locale == "zh-CN" {
        "版本"
    } else {
        "Version"
    };
    let version_text = format!("{version_prefix} {}", app.package_info().version);
    let version = MenuItem::with_id(app, "version", version_text, false, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", tray_label(locale, "show"), true, None::<&str>)?;
    let devtools = MenuItem::with_id(
        app,
        "devtools",
        tray_label(locale, "devtools"),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", tray_label(locale, "quit"), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &devtools, &quit, &version])?;

    let mut builder = TrayIconBuilder::with_id("power-paste-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().0.as_str() {
            "show" => {
                let _ = toggle_panel(app);
            }
            "devtools" => {
                if let Some(window) = app.get_webview_window(PANEL_LABEL) {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                    if !window.is_devtools_open() {
                        window.open_devtools();
                    }
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = toggle_panel(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;

    Ok(())
}
