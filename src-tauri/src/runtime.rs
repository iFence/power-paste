#[cfg(windows)]
use std::sync::Mutex;
use std::sync::{atomic::Ordering, Arc};

use anyhow::{Context, Result};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Position, Size,
    WebviewWindow, WindowEvent,
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
    update::spawn_manual_check,
};

const PANEL_MIN_WIDTH: u32 = 380;
const PANEL_MIN_HEIGHT: u32 = 600;

fn min_panel_physical_size(scale_factor: f64) -> PhysicalSize<u32> {
    let scale_factor = scale_factor.max(1.0);
    PhysicalSize::new(
        (PANEL_MIN_WIDTH as f64 * scale_factor).ceil() as u32,
        (PANEL_MIN_HEIGHT as f64 * scale_factor).ceil() as u32,
    )
}

fn panel_physical_size_from_saved_physical(
    width: u32,
    height: u32,
    saved_scale_factor: f64,
    current_scale_factor: f64,
) -> PhysicalSize<u32> {
    let saved_scale_factor = saved_scale_factor.max(1.0);
    let current_scale_factor = current_scale_factor.max(1.0);
    let logical_width = (width as f64 / saved_scale_factor).round().max(PANEL_MIN_WIDTH as f64);
    let logical_height = (height as f64 / saved_scale_factor)
        .round()
        .max(PANEL_MIN_HEIGHT as f64);

    clamp_panel_size(
        (logical_width * current_scale_factor).ceil() as u32,
        (logical_height * current_scale_factor).ceil() as u32,
        current_scale_factor,
    )
}

fn clamp_panel_logical_size(width: u32, height: u32) -> LogicalSize<f64> {
    LogicalSize::new(
        width.max(PANEL_MIN_WIDTH) as f64,
        height.max(PANEL_MIN_HEIGHT) as f64,
    )
}

fn clamp_panel_size(width: u32, height: u32, scale_factor: f64) -> PhysicalSize<u32> {
    let minimum = min_panel_physical_size(scale_factor);
    PhysicalSize::new(width.max(minimum.width), height.max(minimum.height))
}

fn ensure_panel_min_size(window: &WebviewWindow, scale_factor: f64) -> Result<()> {
    let current = window.outer_size()?;
    let clamped = clamp_panel_size(current.width, current.height, scale_factor);
    if clamped != current {
        window.set_size(Size::Physical(clamped))?;
    }
    Ok(())
}

// Toggles the panel near the cursor and remembers the previous app for later paste-back.
pub(crate) fn toggle_panel(app: &AppHandle) -> Result<()> {
    let window = app
        .get_webview_window(PANEL_LABEL)
        .context("main window not found")?;

    if window.is_visible()? {
        if window.is_focused()? {
            window.hide()?;
        } else {
            remember_last_target_window(app);
            window.show()?;
            window.unminimize()?;
            window.set_focus()?;
        }
    } else {
        remember_last_target_window(app);
        let cursor = app.cursor_position()?;
        let monitor = app.monitor_from_point(cursor.x, cursor.y)?;

        if let Some(monitor) = monitor {
            let current_scale_factor = window.scale_factor()?;
            let target_scale_factor = monitor.scale_factor();
            let should_reapply_size =
                (current_scale_factor - target_scale_factor).abs() > 0.01;

            if let Some(shared) = app.try_state::<Arc<SharedState>>() {
                let settings = shared.settings.lock().unwrap().clone();
                if let (Some(width), Some(height)) =
                    (settings.main_panel_width, settings.main_panel_height)
                {
                    if should_reapply_size {
                        if let Some(saved_scale_factor) = settings.main_panel_scale_factor {
                            let size = panel_physical_size_from_saved_physical(
                                width,
                                height,
                                saved_scale_factor,
                                target_scale_factor,
                            );
                            window.set_size(Size::Physical(size))?;
                        } else {
                            window.set_size(Size::Logical(clamp_panel_logical_size(width, height)))?;
                        }
                    }
                } else {
                    ensure_panel_min_size(&window, target_scale_factor)?;
                }
            } else {
                ensure_panel_min_size(&window, target_scale_factor)?;
            }
            let size = window.outer_size()?;
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
        shared.debug_context_menu_enabled.store(
            crate::should_enable_devtools(settings.debug_enabled),
            Ordering::Relaxed,
        );
        crate::apply_debug_mode(
            &window,
            crate::should_enable_devtools(settings.debug_enabled),
        )?;
        if let (Some(x), Some(y)) = (settings.window_x, settings.window_y) {
            window.set_position(Position::Physical(PhysicalPosition::new(x, y)))?;
        }
        let restored_width = settings.main_panel_width.or(settings.window_width);
        let restored_height = settings.main_panel_height.or(settings.window_height);
        if let (Some(width), Some(height)) = (restored_width, restored_height) {
            if settings.main_panel_width.is_some() && settings.main_panel_height.is_some() {
                if let Some(saved_scale_factor) = settings.main_panel_scale_factor {
                    let size = panel_physical_size_from_saved_physical(
                        width,
                        height,
                        saved_scale_factor,
                        window.scale_factor()?,
                    );
                    window.set_size(Size::Physical(size))?;
                } else {
                    window.set_size(Size::Logical(clamp_panel_logical_size(width, height)))?;
                }
            } else {
                let size = clamp_panel_size(width, height, window.scale_factor()?);
                window.set_size(Size::Physical(size))?;
            }
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
            "show" => "主面板",
            "check_updates" => "检查更新",
            "quit" => "退出",
            _ => "",
        }
    } else {
        match key {
            "show" => "Main Panel",
            "check_updates" => "Check for Updates",
            "quit" => "Quit",
            _ => "",
        }
    }
}

// The tray mirrors the main show/quit actions so the app can stay background-resident.
pub(crate) fn build_tray(app: &AppHandle, locale: &str) -> Result<()> {
    let app_name = app
        .config()
        .product_name
        .clone()
        .unwrap_or_else(|| app.package_info().name.clone());
    let tray_tooltip = format!("{app_name} v{}", app.package_info().version);
    let version_prefix = if locale == "zh-CN" {
        "版本"
    } else {
        "Version"
    };
    let version_text = format!("{version_prefix} {}", app.package_info().version);
    let version = MenuItem::with_id(app, "version", version_text, false, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", tray_label(locale, "show"), true, None::<&str>)?;
    let check_updates = MenuItem::with_id(
        app,
        "check_updates",
        tray_label(locale, "check_updates"),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", tray_label(locale, "quit"), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &check_updates, &quit, &version])?;

    let mut builder = TrayIconBuilder::with_id("power-paste-tray")
        .menu(&menu)
        .tooltip(&tray_tooltip)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().0.as_str() {
            "show" => {
                let _ = toggle_panel(app);
            }
            "check_updates" => {
                let shared = app.state::<Arc<SharedState>>().inner().clone();
                spawn_manual_check(app.clone(), shared);
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
