use std::sync::Arc;

use tauri::{AppHandle, Manager, State};

use crate::{
    clipboard::platform_capabilities,
    installed_apps,
    models::{
        AppError, AppSettings, InstalledAppDto, PlatformCapabilities, SharedState,
        ShortcutStatusDto, WindowSizePayload,
    },
    usecases::{
        execute_reset_settings, execute_retry_shortcut_registration, execute_save_main_panel_size,
        execute_update_settings,
    },
};

// 获取当前应用设置。
#[tauri::command]
pub(crate) fn get_settings(state: State<'_, Arc<SharedState>>) -> Result<AppSettings, AppError> {
    Ok(state.settings.lock().unwrap().clone())
}

// 获取全局快捷键当前注册状态，用于提示快捷键冲突。
#[tauri::command]
pub(crate) fn get_shortcut_status(
    state: State<'_, Arc<SharedState>>,
) -> Result<ShortcutStatusDto, AppError> {
    Ok(state.shortcut_status.lock().unwrap().clone())
}

// 获取当前平台支持能力，供前端控制功能入口。
#[tauri::command]
pub(crate) fn get_platform_capabilities() -> Result<PlatformCapabilities, AppError> {
    Ok(platform_capabilities())
}
// 枚举当前系统已安装应用，供剪贴板忽略应用选择器使用。
// 扫描 /Applications 等目录并逐个读取 Info.plist 属于重 IO，需放入阻塞线程，
// 避免同步阻塞主线程导致设置页卡顿（Mac 端尤其明显）。
#[tauri::command]
pub(crate) async fn list_installed_apps() -> Result<Vec<InstalledAppDto>, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        installed_apps::list_installed_apps().map_err(|error| AppError::Message(error.to_string()))
    })
    .await
    .map_err(|error| AppError::Message(error.to_string()))?
}

// 获取已安装应用图标，用于添加忽略应用规则时按需补充图标。
#[tauri::command]
pub(crate) async fn get_installed_app_icon(
    display_name: String,
    process_name: String,
    app_path: Option<String>,
    bundle_id: Option<String>,
) -> Result<Option<String>, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        installed_apps::installed_app_icon_data_url(display_name, process_name, app_path, bundle_id)
    })
    .await
    .map_err(|error| AppError::Message(error.to_string()))
}

// 更新设置，并同步快捷键、开机启动和调试模式等运行时副作用。
// 内部包含 launchctl 子进程与历史 trim 等重 IO，改为 async 放入阻塞线程，
// 避免同步 command 阻塞主线程导致按钮响应慢。
#[tauri::command]
pub(crate) async fn update_settings(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    payload: AppSettings,
) -> Result<(), AppError> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        execute_update_settings(app, shared, payload)
    })
    .await
    .map_err(|error| AppError::Message(error.to_string()))?
}

// 重新尝试注册当前设置中的全局快捷键。
#[tauri::command]
pub(crate) fn retry_shortcut_registration(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<ShortcutStatusDto, AppError> {
    execute_retry_shortcut_registration(app, state.inner().clone())
}

// 重置设置页可见配置，并保留窗口位置与尺寸。
#[tauri::command]
pub(crate) async fn reset_settings(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<AppSettings, AppError> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        execute_reset_settings(app, shared)
    })
    .await
    .map_err(|error| AppError::Message(error.to_string()))?
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
