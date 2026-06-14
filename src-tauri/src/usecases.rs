use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    apply_debug_mode,
    clipboard::{
        direct_paste_unavailable_reason, platform_capabilities,
        write_item_to_clipboard_with_profile,
    },
    commands::load_item_by_id,
    models::{AppError, AppSettings, SharedState, WindowSizePayload, PANEL_LABEL},
    paste_target::{
        paste_item_to_target, prepare_target_for_paste, resolve_last_target, ResolvedPasteTarget,
    },
    ports::{ClipboardWriterPort, PasteDispatcherPort, SettingsRuntimePort, TargetTrackerPort},
    save_settings,
    shortcuts::{
        register_shortcuts_nonfatal, register_shortcuts_strict, store_and_emit_shortcut_status,
        unregister_configured_shortcuts,
    },
    startup::set_launch_on_startup,
};

struct DefaultClipboardWriter;

impl ClipboardWriterPort for DefaultClipboardWriter {
    fn capabilities(&self) -> crate::models::PlatformCapabilities {
        platform_capabilities()
    }

    fn write_item(
        &self,
        app: &AppHandle,
        item: &crate::models::StoredClipboardItem,
        target: &ResolvedPasteTarget,
    ) -> Result<()> {
        write_item_to_clipboard_with_profile(app, item, target.profile).map(|_| ())
    }
}

struct DefaultPasteDispatcher;

impl PasteDispatcherPort for DefaultPasteDispatcher {
    fn supports_direct_paste(&self) -> bool {
        platform_capabilities().supports_direct_paste
    }

    fn prepare_target(&self, state: &Arc<SharedState>) -> Result<()> {
        prepare_target_for_paste(state)
    }

    fn dispatch_paste(
        &self,
        app: &AppHandle,
        state: &Arc<SharedState>,
        item: &crate::models::StoredClipboardItem,
        target: &ResolvedPasteTarget,
    ) -> Result<bool> {
        paste_item_to_target(app, state, item, target)
    }
}

struct DefaultTargetTracker;

impl TargetTrackerPort for DefaultTargetTracker {
    fn resolve(&self, state: &Arc<SharedState>) -> ResolvedPasteTarget {
        resolve_last_target(state)
    }
}

struct DefaultSettingsRuntime;

impl SettingsRuntimePort for DefaultSettingsRuntime {
    fn apply(
        &self,
        app: &AppHandle,
        state: &Arc<SharedState>,
        settings: &AppSettings,
    ) -> Result<()> {
        let settings = settings.clone().normalized();
        let capabilities = platform_capabilities();
        let previous_settings = state.settings.lock().unwrap().clone();
        let shortcuts_changed = settings.global_shortcut != previous_settings.global_shortcut
            || settings.quick_paste_shortcut != previous_settings.quick_paste_shortcut;

        let next_shortcut_status = if shortcuts_changed {
            unregister_configured_shortcuts(app, &previous_settings);
            match register_shortcuts_strict(app, &settings) {
                Ok(status) => Some(status),
                Err(error) => {
                    unregister_configured_shortcuts(app, &settings);
                    let restored_status = register_shortcuts_nonfatal(app, &previous_settings);
                    store_and_emit_shortcut_status(app, state, restored_status);
                    return Err(error);
                }
            }
        } else {
            None
        };

        if settings.launch_on_startup && !capabilities.supports_launch_on_startup {
            anyhow::bail!("unsupported_launch_on_startup");
        }
        if let Some(path) = settings.lan_transfer_download_dir.as_ref() {
            crate::lan_receiver::validate_download_dir(&PathBuf::from(path))?;
        }
        if capabilities.supports_launch_on_startup {
            set_launch_on_startup(app, settings.launch_on_startup)?;
        }
        save_settings(&state.paths, &settings)?;
        let trimmed_count = {
            let mut store = state.history_store.lock().unwrap();
            store.trim_by_settings(&settings)?
        };
        if trimmed_count > 0 {
            let _ = app.emit(crate::models::HISTORY_UPDATED_EVENT, ());
        }
        state.debug_context_menu_enabled.store(
            crate::should_enable_devtools(settings.debug_enabled),
            std::sync::atomic::Ordering::Relaxed,
        );
        if let Some(window) = app.get_webview_window(PANEL_LABEL) {
            apply_debug_mode(
                &window,
                crate::should_enable_devtools(settings.debug_enabled),
            )?;
        }
        *state.settings.lock().unwrap() = settings.clone();
        if let Some(status) = next_shortcut_status {
            store_and_emit_shortcut_status(app, state, status);
        }
        Ok(())
    }
}

pub(crate) fn execute_retry_shortcut_registration(
    app: AppHandle,
    state: Arc<SharedState>,
) -> Result<crate::models::ShortcutStatusDto, AppError> {
    let settings = state.settings.lock().unwrap().clone();
    unregister_configured_shortcuts(&app, &settings);
    let status = register_shortcuts_nonfatal(&app, &settings);
    store_and_emit_shortcut_status(&app, &state, status.clone());
    Ok(status)
}

pub(crate) fn execute_update_settings(
    app: AppHandle,
    state: Arc<SharedState>,
    payload: AppSettings,
) -> Result<(), AppError> {
    DefaultSettingsRuntime
        .apply(&app, &state, &payload)
        .map_err(AppError::from)
}

pub(crate) fn execute_reset_settings(
    app: AppHandle,
    state: Arc<SharedState>,
) -> Result<AppSettings, AppError> {
    let current = state.settings.lock().unwrap().clone();
    let payload = AppSettings {
        window_x: current.window_x,
        window_y: current.window_y,
        window_width: current.window_width,
        window_height: current.window_height,
        main_panel_width: current.main_panel_width,
        main_panel_height: current.main_panel_height,
        main_panel_scale_factor: current.main_panel_scale_factor,
        ..AppSettings::default()
    };

    DefaultSettingsRuntime
        .apply(&app, &state, &payload)
        .map_err(AppError::from)?;
    Ok(payload.normalized())
}

// 保存主面板宽高，不触发其他设置副作用。
pub(crate) fn execute_save_main_panel_size(
    state: Arc<SharedState>,
    payload: WindowSizePayload,
) -> Result<(), AppError> {
    let mut settings = state.settings.lock().unwrap();
    settings.main_panel_width = Some(payload.width);
    settings.main_panel_height = Some(payload.height);
    settings.main_panel_scale_factor = None;
    save_settings(&state.paths, &settings).map_err(AppError::from)
}

pub(crate) fn execute_copy_item(
    app: AppHandle,
    state: Arc<SharedState>,
    id: String,
) -> Result<(), AppError> {
    let clipboard = DefaultClipboardWriter;
    let capabilities = clipboard.capabilities();
    if !(capabilities.supports_text_write
        || capabilities.supports_html_write
        || capabilities.supports_image_write)
    {
        return Err(AppError::Message("unsupported_clipboard_write".into()));
    }

    let item = load_item_by_id(&state, &id)?;
    let target = DefaultTargetTracker.resolve(&state);
    crate::capture::mark_clipboard_suppressed(&state, item.hash.clone());
    clipboard
        .write_item(&app, &item, &target)
        .map_err(AppError::from)?;
    if state.settings.lock().unwrap().copy_stats_enabled {
        let updated_item = state
            .history_store
            .lock()
            .unwrap()
            .increment_copy_count(&id)
            .map_err(AppError::from)?;
        let _ = app.emit(crate::models::HISTORY_UPDATED_EVENT, updated_item);
    }
    Ok(())
}

pub(crate) fn execute_paste_item(
    app: AppHandle,
    state: Arc<SharedState>,
    id: String,
) -> Result<(), AppError> {
    let paste = DefaultPasteDispatcher;
    if !paste.supports_direct_paste() {
        return Err(AppError::Message(direct_paste_unavailable_reason().into()));
    }

    let item = load_item_by_id(&state, &id)?;

    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        let _ = window.hide();
    }

    let target = DefaultTargetTracker.resolve(&state);
    crate::capture::mark_clipboard_suppressed(&state, item.hash.clone());
    paste.prepare_target(&state).map_err(AppError::from)?;
    paste
        .dispatch_paste(&app, &state, &item, &target)
        .map(|_| ())
        .map_err(AppError::from)?;
    if state.settings.lock().unwrap().paste_stats_enabled {
        let updated_item = state
            .history_store
            .lock()
            .unwrap()
            .increment_paste_count(&id)
            .map_err(AppError::from)?;
        let _ = app.emit(crate::models::HISTORY_UPDATED_EVENT, updated_item);
    }
    Ok(())
}
