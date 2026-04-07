use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::{
    clipboard::platform_capabilities,
    history::history_to_dto,
    models::{AppError, AppSettings, ClipboardItemDto, PlatformCapabilities, SharedState},
    usecases::{execute_copy_item, execute_paste_item, execute_update_settings},
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
    state: State<'_, Arc<SharedState>>,
    payload: AppSettings,
) -> Result<(), AppError> {
    execute_update_settings(app, state.inner().clone(), payload)
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
    state: State<'_, Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    execute_copy_item(app, state.inner().clone(), id)
}

// Paste re-focuses the previous target window, restores clipboard payload, then sends Ctrl+V.
#[tauri::command]
pub(crate) fn paste_item(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    execute_paste_item(app, state.inner().clone(), id)
}

pub(crate) fn load_item_by_id(
    state: &Arc<SharedState>,
    id: &str,
) -> Result<crate::models::StoredClipboardItem, AppError> {
    let store = state.history_store.lock().unwrap();
    let item = store
        .get_item(id)?
        .ok_or_else(|| AppError::Message("Clipboard item not found".into()))?;
    Ok(item)
}
