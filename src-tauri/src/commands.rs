use std::{sync::atomic::Ordering, thread, time::Duration};
use tauri::{AppHandle, Manager, State};
#[cfg(windows)]
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::{
    clipboard::platform_capabilities,
    apply_debug_mode,
    clipboard::write_item_to_clipboard_with_profile,
    capture::mark_clipboard_suppressed,
    history::history_to_dto,
    models::{
        AppError, AppSettings, ClipboardItemDto, PlatformCapabilities, SharedState, PANEL_LABEL,
    },
    paste_target::{
        focus_last_target_window, last_target_profile, paste_mixed_item_for_profile,
        send_native_paste_shortcut, wait_for_paste_target_focus,
    },
    save_settings,
    startup::set_launch_on_startup,
};

// History queries always read from in-memory state; persistence is handled on writes.
#[tauri::command]
pub(crate) fn get_history(
    state: State<'_, std::sync::Arc<SharedState>>,
    query: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<ClipboardItemDto>, AppError> {
    let store = state.history_store.lock().unwrap();
    let history = store.list_history(query.as_deref(), limit.unwrap_or(500))?;
    Ok(history_to_dto(
        &history,
        query.as_deref(),
        limit.unwrap_or(500),
    ))
}

#[tauri::command]
pub(crate) fn get_settings(
    state: State<'_, std::sync::Arc<SharedState>>,
) -> Result<AppSettings, AppError> {
    Ok(state.settings.lock().unwrap().clone())
}

#[tauri::command]
pub(crate) fn get_platform_capabilities() -> Result<PlatformCapabilities, AppError> {
    Ok(platform_capabilities())
}

// Settings updates also need to fan out to side effects like shortcut registration and startup.
#[tauri::command]
pub(crate) fn update_settings(
    app: AppHandle,
    state: State<'_, std::sync::Arc<SharedState>>,
    payload: AppSettings,
) -> Result<(), AppError> {
    let previous_shortcut = state.settings.lock().unwrap().global_shortcut.clone();

    #[cfg(windows)]
    {
        let manager = app.global_shortcut();
        if let Ok(shortcut) = previous_shortcut.parse::<Shortcut>() {
            let _ = manager.unregister(shortcut);
        }
        if !payload.global_shortcut.trim().is_empty() {
            let shortcut = payload
                .global_shortcut
                .parse::<Shortcut>()
                .map_err(|error| AppError::Message(format!("Invalid shortcut: {error}")))?;
            manager
                .register(shortcut)
                .map_err(|error| AppError::Message(error.to_string()))?;
        }
    }

    set_launch_on_startup(&app, payload.launch_on_startup)?;
    save_settings(&state.paths, &payload)?;
    state
        .debug_context_menu_enabled
        .store(payload.debug_enabled, Ordering::Relaxed);
    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        apply_debug_mode(&window, payload.debug_enabled)?;
    }
    *state.settings.lock().unwrap() = payload;
    Ok(())
}

#[tauri::command]
pub(crate) fn toggle_pin(
    state: State<'_, std::sync::Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.toggle_pin(&id)?;
    *state.history.lock().unwrap() = store.list_all()?;
    Ok(())
}

#[tauri::command]
pub(crate) fn toggle_favorite(
    state: State<'_, std::sync::Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.toggle_favorite(&id)?;
    *state.history.lock().unwrap() = store.list_all()?;
    Ok(())
}

#[tauri::command]
pub(crate) fn delete_item(
    state: State<'_, std::sync::Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.delete_item(&id)?;
    *state.history.lock().unwrap() = store.list_all()?;
    Ok(())
}

#[tauri::command]
pub(crate) fn update_text_item(
    state: State<'_, std::sync::Arc<SharedState>>,
    id: String,
    text: String,
) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.update_text_item(&id, &text)?;
    *state.history.lock().unwrap() = store.list_all()?;
    Ok(())
}

#[tauri::command]
pub(crate) fn clear_history(state: State<'_, std::sync::Arc<SharedState>>) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.clear_history()?;
    *state.history.lock().unwrap() = store.list_all()?;
    Ok(())
}

// Copy writes the payload back to the system clipboard but does not trigger paste.
#[tauri::command]
pub(crate) fn copy_item(
    app: AppHandle,
    state: State<'_, std::sync::Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    let capabilities = platform_capabilities();
    if !(capabilities.supports_text_write
        || capabilities.supports_html_write
        || capabilities.supports_image_write)
    {
        return Err(AppError::Message("unsupported_clipboard_write".into()));
    }

    let store = state.history_store.lock().unwrap();
    let item = store
        .get_item(&id)?
        .ok_or_else(|| AppError::Message("Clipboard item not found".into()))?;
    drop(store);

    let shared = state.inner().clone();
    let profile = last_target_profile(&shared);
    mark_clipboard_suppressed(&shared, item.hash.clone());
    write_item_to_clipboard_with_profile(&app, &item, profile)?;
    Ok(())
}

// Paste re-focuses the previous target window, restores clipboard payload, then sends Ctrl+V.
#[tauri::command]
pub(crate) fn paste_item(
    app: AppHandle,
    state: State<'_, std::sync::Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    if !platform_capabilities().supports_direct_paste {
        return Err(AppError::Message("unsupported_direct_paste".into()));
    }

    let store = state.history_store.lock().unwrap();
    let item = store
        .get_item(&id)?
        .ok_or_else(|| AppError::Message("Clipboard item not found".into()))?;
    drop(store);

    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        let _ = window.hide();
    }

    let shared = state.inner().clone();
    let profile = last_target_profile(&shared);
    mark_clipboard_suppressed(&shared, item.hash.clone());
    focus_last_target_window(&shared);
    wait_for_paste_target_focus(&shared);
    if paste_mixed_item_for_profile(&app, &shared, &item, profile)? {
        return Ok(());
    }
    write_item_to_clipboard_with_profile(&app, &item, profile)?;
    thread::sleep(Duration::from_millis(180));
    send_native_paste_shortcut(&shared)?;
    Ok(())
}
