use std::sync::Arc;

use tauri::State;

use crate::{
    history::history_to_dto,
    models::{AppError, ClipboardHistoryPageDto, SharedState},
};

// 历史记录按页从数据库读取，避免记录较多时一次性加载全部内容。
#[tauri::command]
pub(crate) fn get_history(
    state: State<'_, Arc<SharedState>>,
    query: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<ClipboardHistoryPageDto, AppError> {
    let limit = limit.unwrap_or(500);
    let offset = offset.unwrap_or(0);
    let store = state.history_store.lock().unwrap();
    let total_count = store.count_history(query.as_deref())?;
    let history = store.list_history(query.as_deref(), limit, offset)?;
    Ok(ClipboardHistoryPageDto {
        items: history_to_dto(&history, query.as_deref(), limit),
        total_count,
    })
}

// 切换历史条目的置顶状态。
#[tauri::command]
pub(crate) fn toggle_pin(state: State<'_, Arc<SharedState>>, id: String) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.toggle_pin(&id)?;
    Ok(())
}

// 切换历史条目的收藏状态。
#[tauri::command]
pub(crate) fn toggle_favorite(
    state: State<'_, Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.toggle_favorite(&id)?;
    Ok(())
}

// 删除指定历史条目。
#[tauri::command]
pub(crate) fn delete_item(state: State<'_, Arc<SharedState>>, id: String) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.delete_item(&id)?;
    Ok(())
}

// 更新纯文本历史条目的内容。
#[tauri::command]
pub(crate) fn update_text_item(
    state: State<'_, Arc<SharedState>>,
    id: String,
    text: String,
) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.update_text_item(&id, &text)?;
    Ok(())
}

// 更新指定历史条目的标签颜色，最多保留三个预设颜色标签。
#[tauri::command]
pub(crate) fn update_item_tags(
    state: State<'_, Arc<SharedState>>,
    id: String,
    tag_colors: Vec<String>,
) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.update_item_tags(&id, &tag_colors)?;
    Ok(())
}

// 清空可删除的历史记录，并重置剪贴板观察状态。
#[tauri::command]
pub(crate) fn clear_history(state: State<'_, Arc<SharedState>>) -> Result<(), AppError> {
    let store = state.history_store.lock().unwrap();
    store.clear_history()?;
    crate::capture::reset_clipboard_observation(&state);
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
