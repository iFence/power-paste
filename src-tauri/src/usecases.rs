use std::sync::Arc;

use anyhow::Result;
use tauri::{AppHandle, Manager};
#[cfg(windows)]
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::{
    apply_debug_mode,
    clipboard::{platform_capabilities, write_item_to_clipboard_with_profile},
    commands::load_item_by_id,
    models::{AppError, AppSettings, SharedState, PANEL_LABEL},
    paste_target::{
        paste_item_to_target, prepare_target_for_paste, resolve_last_target, ResolvedPasteTarget,
    },
    ports::{ClipboardWriterPort, PasteDispatcherPort, SettingsRuntimePort, TargetTrackerPort},
    save_settings,
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
        write_item_to_clipboard_with_profile(app, item, target.profile)
    }
}

struct DefaultPasteDispatcher;

impl PasteDispatcherPort for DefaultPasteDispatcher {
    fn supports_direct_paste(&self) -> bool {
        platform_capabilities().supports_direct_paste
    }

    fn prepare_target(&self, state: &Arc<SharedState>) {
        prepare_target_for_paste(state);
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
        let previous_shortcut = state.settings.lock().unwrap().global_shortcut.clone();

        #[cfg(windows)]
        {
            let manager = app.global_shortcut();
            if let Ok(shortcut) = previous_shortcut.parse::<Shortcut>() {
                let _ = manager.unregister(shortcut);
            }
            if !settings.global_shortcut.trim().is_empty() {
                let shortcut = settings
                    .global_shortcut
                    .parse::<Shortcut>()
                    .map_err(|error| anyhow::anyhow!("Invalid shortcut: {error}"))?;
                manager.register(shortcut)?;
            }
        }

        set_launch_on_startup(app, settings.launch_on_startup)?;
        save_settings(&state.paths, settings)?;
        state
            .debug_context_menu_enabled
            .store(settings.debug_enabled, std::sync::atomic::Ordering::Relaxed);
        if let Some(window) = app.get_webview_window(PANEL_LABEL) {
            apply_debug_mode(&window, settings.debug_enabled)?;
        }
        *state.settings.lock().unwrap() = settings.clone();
        Ok(())
    }
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
        .map_err(AppError::from)
}

pub(crate) fn execute_paste_item(
    app: AppHandle,
    state: Arc<SharedState>,
    id: String,
) -> Result<(), AppError> {
    let paste = DefaultPasteDispatcher;
    if !paste.supports_direct_paste() {
        return Err(AppError::Message("unsupported_direct_paste".into()));
    }

    let item = load_item_by_id(&state, &id)?;

    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        let _ = window.hide();
    }

    let target = DefaultTargetTracker.resolve(&state);
    crate::capture::mark_clipboard_suppressed(&state, item.hash.clone());
    paste.prepare_target(&state);
    paste
        .dispatch_paste(&app, &state, &item, &target)
        .map(|_| ())
        .map_err(AppError::from)
}
