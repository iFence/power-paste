use std::{path::Path, sync::Arc};

use tauri::{AppHandle, Manager, State};

use crate::{
    clipboard::platform_capabilities,
    history::history_to_dto,
    history::normalize_link_url,
    models::{
        AppError, AppSettings, ClipboardItemDto, LanReceiverStateDto, PlatformCapabilities,
        SharedState,
    },
    usecases::{execute_copy_item, execute_paste_item, execute_update_settings},
};

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::ShellExecuteW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

// 历史记录按页从数据库读取，避免记录较多时一次性加载全部内容。
#[tauri::command]
pub(crate) fn get_history(
    state: State<'_, std::sync::Arc<SharedState>>,
    query: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<ClipboardItemDto>, AppError> {
    let limit = limit.unwrap_or(500);
    let offset = offset.unwrap_or(0);
    let store = state.history_store.lock().unwrap();
    let history = store.list_history(query.as_deref(), limit, offset)?;
    Ok(history_to_dto(&history, query.as_deref(), limit))
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
    crate::capture::reset_clipboard_observation(&state);
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

// 使用系统默认浏览器打开链接，仅允许已识别的网页链接格式。
#[tauri::command]
pub(crate) fn open_external_url(url: String) -> Result<(), AppError> {
    let normalized =
        normalize_link_url(&url).ok_or_else(|| AppError::Message("invalid_url".into()))?;

    #[cfg(target_os = "windows")]
    {
        let operation: Vec<u16> = "open\0".encode_utf16().collect();
        let target: Vec<u16> = format!("{normalized}\0").encode_utf16().collect();
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };

        if result as usize <= 32 {
            return Err(anyhow::anyhow!("failed to open external url: {normalized}").into());
        }
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&normalized)
            .spawn()
            .map_err(anyhow::Error::from)?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&normalized)
            .spawn()
            .map_err(anyhow::Error::from)?;
    }

    Ok(())
}

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

// 获取系统默认下载目录，用于互传文件保存位置的默认展示。
#[tauri::command]
pub(crate) fn get_default_download_dir(app: AppHandle) -> Result<String, AppError> {
    let dir = app
        .path()
        .download_dir()
        .map_err(|error| AppError::Message(error.to_string()))?;
    Ok(dir.to_string_lossy().to_string())
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
    file_name: String,
    mime_type: Option<String>,
    bytes: Vec<u8>,
) -> Result<LanReceiverStateDto, AppError> {
    crate::lan_receiver::send_desktop_file(app, state.inner().clone(), file_name, mime_type, bytes)
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

    #[cfg(target_os = "windows")]
    {
        let operation: Vec<u16> = "open\0".encode_utf16().collect();
        let target: Vec<u16> = format!("{}\0", path.to_string_lossy())
            .encode_utf16()
            .collect();
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };

        if result as usize <= 32 {
            return Err(anyhow::anyhow!("failed to open local file: {}", path.display()).into());
        }
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|error| AppError::Message(error.to_string()))?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|error| AppError::Message(error.to_string()))?;
    }

    Ok(())
}

fn reveal_local_path(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::Message("lan_transfer_file_not_found".into()));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", path.to_string_lossy()))
            .spawn()
            .map_err(|error| AppError::Message(error.to_string()))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map_err(|error| AppError::Message(error.to_string()))?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let parent = path.parent().unwrap_or(path);
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|error| AppError::Message(error.to_string()))?;
    }

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
