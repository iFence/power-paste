use std::{path::Path, sync::Arc};

use tauri::{AppHandle, State};

use crate::{
    models::{AppError, LanReceiverStateDto, SharedState},
    system_open,
};

// 启动局域网接收服务，返回手机扫码访问地址、二维码和会话过期时间。
#[tauri::command]
pub(crate) fn start_lan_receiver(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<LanReceiverStateDto, AppError> {
    crate::lan_receiver::start(app, state.inner().clone())
}

// 停止当前局域网接收服务，使已生成二维码和令牌立即失效。
#[tauri::command]
pub(crate) fn stop_lan_receiver(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<LanReceiverStateDto, AppError> {
    crate::lan_receiver::stop(app, state.inner().clone())
}

// 获取当前局域网接收服务状态，用于前端恢复二维码弹窗。
#[tauri::command]
pub(crate) fn get_lan_receiver_state(
    state: State<'_, Arc<SharedState>>,
) -> Result<LanReceiverStateDto, AppError> {
    Ok(crate::lan_receiver::get_state(state.inner()))
}

// 电脑端向当前互传会话发送文本消息，手机端会显示并提供复制。
#[tauri::command]
pub(crate) fn send_lan_transfer_text(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    text: String,
) -> Result<LanReceiverStateDto, AppError> {
    crate::lan_receiver::send_desktop_text(app, state.inner().clone(), text)
}

// 电脑端向当前互传会话发送文件或图片，手机端会显示下载链接。
#[tauri::command]
pub(crate) fn send_lan_transfer_file(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    path: String,
    file_name: Option<String>,
    mime_type: Option<String>,
) -> Result<LanReceiverStateDto, AppError> {
    crate::lan_receiver::send_desktop_file(app, state.inner().clone(), path, file_name, mime_type)
}

// 打开互传消息对应的本地文件，仅允许打开当前会话中记录过的文件。
#[tauri::command]
pub(crate) fn open_lan_transfer_file(
    state: State<'_, Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    let path = crate::lan_receiver::message_local_path(state.inner(), &id)?;
    open_local_path(&path)
}

// 在系统文件管理器中定位互传消息对应的本地文件。
#[tauri::command]
pub(crate) fn reveal_lan_transfer_file(
    state: State<'_, Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    let path = crate::lan_receiver::message_local_path(state.inner(), &id)?;
    reveal_local_path(&path)
}

fn open_local_path(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::Message("lan_transfer_file_not_found".into()));
    }

    system_open::open_path(path).map_err(AppError::from)
}

fn reveal_local_path(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::Message("lan_transfer_file_not_found".into()));
    }

    system_open::reveal_path(path).map_err(AppError::from)
}
