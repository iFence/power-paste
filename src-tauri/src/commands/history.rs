use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::models::{AppError, ClipboardHistoryPageDto, HistoryQueryPayload, SharedState};

// 历史记录按页从数据库读取，避免记录较多时一次性加载全部内容。
#[tauri::command]
pub(crate) fn get_history(
    state: State<'_, Arc<SharedState>>,
    payload: Option<HistoryQueryPayload>,
) -> Result<ClipboardHistoryPageDto, AppError> {
    let mut payload = payload.unwrap_or_default();
    let (copy_stats_enabled, paste_stats_enabled) = {
        let settings = state.settings.lock().unwrap();
        (
            settings.copy_stats_enabled && !settings.paste_stats_enabled,
            settings.paste_stats_enabled,
        )
    };
    payload.copy_stats_enabled = copy_stats_enabled;
    payload.paste_stats_enabled = paste_stats_enabled;
    let limit = payload.limit.unwrap_or(500);
    let offset = payload.offset.unwrap_or(0);
    let store = state.history_store.lock().unwrap();
    let total_count = store.count_history(&payload)?;
    let history = store.list_history(&payload, limit, offset)?;
    Ok(ClipboardHistoryPageDto {
        items: history,
        total_count,
    })
}

// 切换历史条目的置顶状态。
#[tauri::command]
pub(crate) fn toggle_pin(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.toggle_pin(&id)?;
    crate::sync::schedule_auto_sync(app, state.inner().clone());
    Ok(())
}

// 切换历史条目的收藏状态。
#[tauri::command]
pub(crate) fn toggle_favorite(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.toggle_favorite(&id)?;
    crate::sync::schedule_auto_sync(app, state.inner().clone());
    Ok(())
}

// 删除指定历史条目。
#[tauri::command]
pub(crate) fn delete_item(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.delete_item(&id)?;
    crate::sync::schedule_auto_sync(app, state.inner().clone());
    Ok(())
}

// 更新纯文本历史条目的内容。
#[tauri::command]
pub(crate) fn update_text_item(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    id: String,
    text: String,
) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.update_text_item(&id, &text)?;
    crate::sync::schedule_auto_sync(app, state.inner().clone());
    Ok(())
}

// 更新指定历史条目的标签颜色，最多保留三个预设颜色标签。
#[tauri::command]
pub(crate) fn update_item_tags(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    id: String,
    tag_colors: Vec<String>,
) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.update_item_tags(&id, &tag_colors)?;
    crate::sync::schedule_auto_sync(app, state.inner().clone());
    Ok(())
}

// 清空可删除的历史记录，并重置剪贴板观察状态。
#[tauri::command]
pub(crate) fn clear_history(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.clear_history()?;
    crate::capture::reset_clipboard_observation(&state);
    crate::sync::schedule_auto_sync(app, state.inner().clone());
    Ok(())
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
