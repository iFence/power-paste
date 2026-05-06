use std::sync::Arc;

use tauri::{AppHandle, Manager, State};

use crate::{
    clipboard::platform_capabilities,
    models::{AppError, AppSettings, PlatformCapabilities, SharedState, WindowSizePayload},
    usecases::{execute_reset_settings, execute_save_main_panel_size, execute_update_settings},
};

// 获取当前应用设置。
#[tauri::command]
pub(crate) fn get_settings(state: State<'_, Arc<SharedState>>) -> Result<AppSettings, AppError> {
    Ok(state.settings.lock().unwrap().clone())
}

// 获取当前平台支持能力，供前端控制功能入口。
#[tauri::command]
pub(crate) fn get_platform_capabilities() -> Result<PlatformCapabilities, AppError> {
    Ok(platform_capabilities())
}

// 更新设置，并同步快捷键、开机启动和调试模式等运行时副作用。
#[tauri::command]
pub(crate) fn update_settings(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    payload: AppSettings,
) -> Result<(), AppError> {
    execute_update_settings(app, state.inner().clone(), payload)
}

// 重置设置页可见配置，并保留窗口位置与尺寸。
#[tauri::command]
pub(crate) fn reset_settings(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<AppSettings, AppError> {
    execute_reset_settings(app, state.inner().clone())
}

// 获取系统默认下载目录，用于互传文件保存位置的默认展示。
#[tauri::command]
pub(crate) fn get_default_download_dir(app: AppHandle) -> Result<String, AppError> {
    let dir = app
        .path()
        .download_dir()
        .map_err(|error| AppError::Message(error.to_string()))?;
    Ok(dir.to_string_lossy().to_string())
}

// 保存主面板最近一次使用的宽高，避免设置页尺寸覆盖主面板恢复尺寸。
#[tauri::command]
pub(crate) fn save_main_panel_size(
    state: State<'_, Arc<SharedState>>,
    payload: WindowSizePayload,
) -> Result<(), AppError> {
    execute_save_main_panel_size(state.inner().clone(), payload)
}
