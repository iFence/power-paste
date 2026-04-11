use std::sync::{atomic::AtomicBool, Arc, Mutex};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
use anyhow::Context;
use anyhow::Result;
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{Shortcut, ShortcutState};
#[cfg(windows)]
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
#[cfg(windows)]
use windows_core::Interface;

mod capture;
mod clipboard;
mod clipboard_html;
mod commands;
mod history;
mod models;
mod paste_target;
mod ports;
mod repository;
mod rich_text;
mod runtime;
mod startup;
mod storage;
mod update;
mod usecases;

// Tauri command entrypoints stay thin and delegate to feature modules.
use commands::{
    clear_history, copy_item, delete_item, get_history, get_platform_capabilities, get_settings,
    open_external_url, paste_item, toggle_favorite, toggle_pin, update_settings, update_text_item,
};
use models::{
    MonitorState, SharedState, StoragePaths, UpdateStatus, DEBUG_CONTEXT_MENU_INIT_SCRIPT,
};
use repository::SqliteHistoryStore;
use runtime::{configure_window, toggle_panel};
use startup::set_launch_on_startup;
use storage::{load_settings, save_settings};

#[cfg(windows)]
fn powershell(script: &str) -> Result<String> {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-STA", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .context("failed to execute powershell")?;

    if !output.status.success() {
        anyhow::bail!(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// Keeps the frontend debug switches and the native WebView settings in sync.
fn apply_debug_mode(window: &tauri::WebviewWindow, enabled: bool) -> Result<()> {
    window.eval(format!(
        "window.__CLIPDESK_DEBUG_GUARD__ = Object.assign(window.__CLIPDESK_DEBUG_GUARD__ || {{}}, {{ allowContextMenu: {} }});",
        if enabled { "true" } else { "false" }
    ))?;

    if !enabled && window.is_devtools_open() {
        window.close_devtools();
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
                    let settings = unsafe { webview.Settings() }
                        .context("failed to access webview settings")?;

                    unsafe {
                        settings.SetAreDevToolsEnabled(enabled)?;
                        settings.SetAreDefaultContextMenusEnabled(true)?;
                    }

                    if let Ok(settings3) = settings.cast::<ICoreWebView2Settings3>() {
                        unsafe {
                            settings3.SetAreBrowserAcceleratorKeysEnabled(enabled)?;
                        }
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

    Ok(())
}

// The crate root only assembles modules, shared state and Tauri plugins.
pub fn run() {
    tauri::Builder::default()
        .append_invoke_initialization_script(DEBUG_CONTEXT_MENU_INIT_SCRIPT)
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = toggle_panel(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None::<Vec<&'static str>>,
        ))
        .plugin(tauri_plugin_clipboard_next::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state == ShortcutState::Released {
                        let _ = toggle_panel(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let root = app.path().app_local_data_dir()?;
            let paths = StoragePaths::new(root)?;
            let settings = Arc::new(Mutex::new(load_settings(&paths).unwrap_or_default()));
            let history_store = SqliteHistoryStore::new(&paths)?;
            let history = Arc::new(Mutex::new(history_store.list_all()?));
            let history_store = Arc::new(Mutex::new(history_store));

            let shared = Arc::new(SharedState {
                paths,
                settings: settings.clone(),
                history_store: history_store.clone(),
                history: history.clone(),
                monitor: Arc::new(Mutex::new(MonitorState::default())),
                debug_context_menu_enabled: Arc::new(AtomicBool::new(
                    settings.lock().unwrap().debug_enabled,
                )),
                macos_direct_paste_permission_verified: Arc::new(AtomicBool::new(false)),
                update_status: Arc::new(Mutex::new(UpdateStatus::idle(
                    app.package_info().version.to_string(),
                ))),
                pending_update: Arc::new(Mutex::new(None)),
                update_debug_override: Arc::new(Mutex::new(None)),
            });

            let launch_on_startup = settings.lock().unwrap().launch_on_startup;
            let _ = set_launch_on_startup(app.handle(), launch_on_startup);

            configure_window(app.handle(), shared.clone())?;
            let locale = settings.lock().unwrap().locale.clone();
            runtime::build_tray(app.handle(), &locale)?;

            {
                use tauri_plugin_global_shortcut::GlobalShortcutExt;
                let shortcut = shared.settings.lock().unwrap().global_shortcut.clone();
                if let Ok(shortcut) = shortcut.parse::<Shortcut>() {
                    app.global_shortcut().register(shortcut)?;
                }
            }

            capture::start_clipboard_monitor(app.handle().clone(), shared.clone());
            app.manage(shared);
            update::spawn_startup_check(
                app.handle().clone(),
                app.state::<Arc<SharedState>>().inner().clone(),
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_history,
            get_platform_capabilities,
            get_settings,
            update_settings,
            toggle_pin,
            toggle_favorite,
            delete_item,
            update_text_item,
            clear_history,
            copy_item,
            paste_item,
            open_external_url,
            update::get_update_state,
            update::check_for_updates,
            update::install_update,
            update::set_update_debug_state
        ])
        .run(tauri::generate_context!())
        .expect("error while running Power Paste");
}
