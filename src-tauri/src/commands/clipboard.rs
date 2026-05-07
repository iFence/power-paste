use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::{
    history::normalize_link_url,
    models::{AppError, SharedState},
    system_open,
    usecases::{execute_copy_item, execute_paste_item},
};

// 将指定历史条目写回系统剪贴板，但不触发粘贴。
#[tauri::command]
pub(crate) fn copy_item(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    execute_copy_item(app, state.inner().clone(), id)
}

// 将指定历史条目写回剪贴板，并粘贴到上一个目标窗口。
#[tauri::command]
pub(crate) fn paste_item(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    execute_paste_item(app, state.inner().clone(), id)
}

// 使用系统默认浏览器打开链接，仅允许已识别的网页链接格式。
#[tauri::command]
pub(crate) fn open_external_url(url: String) -> Result<(), AppError> {
    let normalized =
        normalize_link_url(&url).ok_or_else(|| AppError::Message("invalid_url".into()))?;
    system_open::open_url(&normalized).map_err(AppError::from)
}
