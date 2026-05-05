use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::{
    history::normalize_link_url,
    models::{AppError, SharedState},
    usecases::{execute_copy_item, execute_paste_item},
};

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::ShellExecuteW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

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
