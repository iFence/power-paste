use std::{fs, sync::atomic::Ordering, thread, time::Duration};

use chrono::Utc;
use tauri::{AppHandle, Manager, State};
#[cfg(windows)]
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::{
    apply_debug_mode,
    capture::mark_clipboard_suppressed,
    clipboard_write::write_item_to_clipboard_with_profile,
    history::{history_to_dto, sort_history},
    models::{
        AppError, AppSettings, ClipboardItemDto, PlatformCapabilities, SharedState, PANEL_LABEL,
    },
    paste_target::{
        focus_last_target_window, last_target_profile, paste_mixed_item_for_profile,
        send_native_paste_shortcut, wait_for_paste_target_focus,
    },
    preview_text, save_history, save_settings, sha256_hex,
    startup::set_launch_on_startup,
};

fn platform_capabilities() -> PlatformCapabilities {
    PlatformCapabilities {
        platform: std::env::consts::OS.to_string(),
        supports_clipboard_write: cfg!(windows) || cfg!(target_os = "macos"),
        supports_direct_paste: cfg!(windows) || cfg!(target_os = "macos"),
        supports_launch_on_startup: cfg!(windows) || cfg!(target_os = "macos"),
        supports_mixed_replay: cfg!(windows),
    }
}

// History queries always read from in-memory state; persistence is handled on writes.
#[tauri::command]
pub(crate) fn get_history(
    state: State<'_, std::sync::Arc<SharedState>>,
    query: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<ClipboardItemDto>, AppError> {
    let history = state.history.lock().unwrap();
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

    set_launch_on_startup(payload.launch_on_startup)?;
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
    let mut history = state.history.lock().unwrap();
    if let Some(item) = history.iter_mut().find(|item| item.id == id) {
        item.pinned = !item.pinned;
        item.pinned_at = item.pinned.then(|| Utc::now().to_rfc3339());
    }
    history.sort_by(sort_history);
    save_history(&state.paths, &history)?;
    Ok(())
}

#[tauri::command]
pub(crate) fn toggle_favorite(
    state: State<'_, std::sync::Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    let mut history = state.history.lock().unwrap();
    if let Some(item) = history.iter_mut().find(|item| item.id == id) {
        item.favorite = !item.favorite;
    }
    history.sort_by(sort_history);
    save_history(&state.paths, &history)?;
    Ok(())
}

#[tauri::command]
pub(crate) fn delete_item(
    state: State<'_, std::sync::Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    let mut history = state.history.lock().unwrap();
    if let Some(index) = history.iter().position(|item| item.id == id) {
        if let Some(image_path) = history[index].image_path.clone() {
            let _ = fs::remove_file(image_path);
        }
        history.remove(index);
        save_history(&state.paths, &history)?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn update_text_item(
    state: State<'_, std::sync::Arc<SharedState>>,
    id: String,
    text: String,
) -> Result<(), AppError> {
    let mut history = state.history.lock().unwrap();
    let item = history
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or_else(|| AppError::Message("Clipboard item not found".into()))?;

    if item.kind != "text" {
        return Err(AppError::Message("Only text items can be edited".into()));
    }

    item.full_text = Some(text.clone());
    item.html_text = None;
    item.rtf_text = None;
    item.preview = preview_text(&text);
    item.hash = sha256_hex(text.as_bytes());
    item.created_at = Utc::now().to_rfc3339();
    save_history(&state.paths, &history)?;
    Ok(())
}

#[tauri::command]
pub(crate) fn clear_history(state: State<'_, std::sync::Arc<SharedState>>) -> Result<(), AppError> {
    let mut history = state.history.lock().unwrap();
    for item in history.iter().filter(|item| !item.pinned) {
        if let Some(image_path) = &item.image_path {
            let _ = fs::remove_file(image_path);
        }
    }
    history.retain(|item| item.pinned);
    save_history(&state.paths, &history)?;
    Ok(())
}

// Copy writes the payload back to the system clipboard but does not trigger paste.
#[tauri::command]
pub(crate) fn copy_item(
    state: State<'_, std::sync::Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    if !cfg!(windows) && !cfg!(target_os = "macos") {
        return Err(AppError::Message("unsupported_clipboard_write".into()));
    }

    let history = state.history.lock().unwrap();
    let item = history
        .iter()
        .find(|item| item.id == id)
        .cloned()
        .ok_or_else(|| AppError::Message("Clipboard item not found".into()))?;
    drop(history);

    let shared = state.inner().clone();
    let profile = last_target_profile(&shared);
    mark_clipboard_suppressed(&shared, item.hash.clone());
    write_item_to_clipboard_with_profile(&item, profile)?;
    Ok(())
}

// Paste re-focuses the previous target window, restores clipboard payload, then sends Ctrl+V.
#[tauri::command]
pub(crate) fn paste_item(
    app: AppHandle,
    state: State<'_, std::sync::Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    if !cfg!(windows) && !cfg!(target_os = "macos") {
        return Err(AppError::Message("unsupported_direct_paste".into()));
    }

    let history = state.history.lock().unwrap();
    let item = history
        .iter()
        .find(|item| item.id == id)
        .cloned()
        .ok_or_else(|| AppError::Message("Clipboard item not found".into()))?;
    drop(history);

    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        let _ = window.hide();
    }

    let shared = state.inner().clone();
    let profile = last_target_profile(&shared);
    mark_clipboard_suppressed(&shared, item.hash.clone());
    focus_last_target_window(&shared);
    wait_for_paste_target_focus(&shared);
    if paste_mixed_item_for_profile(&shared, &item, profile)? {
        return Ok(());
    }
    write_item_to_clipboard_with_profile(&item, profile)?;
    thread::sleep(Duration::from_millis(180));
    send_native_paste_shortcut(&shared)?;
    Ok(())
}
