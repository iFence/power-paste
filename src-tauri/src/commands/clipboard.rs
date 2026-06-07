use std::{fs, path::PathBuf, sync::Arc};

use anyhow::Context;
use tauri::{AppHandle, State};

use crate::{
    commands::history::load_item_by_id,
    history::normalize_link_url,
    models::{AppError, SharedState},
    system_open,
    usecases::{execute_copy_item, execute_paste_item},
};

fn drag_temp_dir() -> PathBuf {
    std::env::temp_dir().join("power-paste-drag")
}

fn drag_image_file_name(id: &str) -> String {
    let safe_id = id
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || value == '-' || value == '_' {
                value
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("{safe_id}.png")
}

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

// 将图片历史条目写入临时 PNG 文件，供系统拖拽使用；参数 id 为历史条目 ID。
#[tauri::command]
pub(crate) fn prepare_image_drag_file(
    state: State<'_, Arc<SharedState>>,
    id: String,
) -> Result<String, AppError> {
    let item = load_item_by_id(state.inner(), &id)?;
    let png_bytes = item
        .image_png
        .as_ref()
        .or(item.image_preview_png.as_ref())
        .filter(|bytes| !bytes.is_empty())
        .ok_or_else(|| AppError::Message("image_drag_file_unavailable".into()))?;

    let dir = drag_temp_dir();
    fs::create_dir_all(&dir)
        .context("failed to create image drag temp directory")
        .map_err(AppError::from)?;
    let path = dir.join(drag_image_file_name(&id));
    fs::write(&path, png_bytes)
        .context("failed to write image drag temp file")
        .map_err(AppError::from)?;
    Ok(path.to_string_lossy().to_string())
}

// 使用系统默认浏览器打开链接，仅允许已识别的网页链接格式。
#[tauri::command]
pub(crate) fn open_external_url(url: String) -> Result<(), AppError> {
    let normalized =
        normalize_link_url(&url).ok_or_else(|| AppError::Message("invalid_url".into()))?;
    system_open::open_url(&normalized).map_err(AppError::from)
}
