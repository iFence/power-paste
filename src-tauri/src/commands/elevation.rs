use tauri::AppHandle;

use crate::models::AppError;

// 以管理员身份重新启动 Power Paste，用于向管理员窗口自动粘贴。
#[tauri::command]
pub(crate) fn restart_as_admin(app: AppHandle) -> Result<(), AppError> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

        let executable = std::env::current_exe()
            .map_err(|error| AppError::Message(error.to_string()))?
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let operation = "runas"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();

        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                executable.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if result as isize <= 32 {
            return Err(AppError::Message("restart_as_admin_failed".into()));
        }

        app.exit(0);
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        let _ = app;
        Err(AppError::Message("unsupported_restart_as_admin".into()))
    }
}
