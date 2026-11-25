use tauri::AppHandle;

use crate::models::AppError;

const RESTART_AS_ADMIN_ARG_PREFIX: &str = "--restart-as-admin=";
const RESTART_PARENT_WAIT_MS: u32 = 5_000;

// 等待提权重启前的旧实例完全退出，避免两个实例同时占用单实例锁和全局快捷键。
#[cfg(windows)]
pub(crate) fn wait_for_restart_parent() {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        Storage::FileSystem::SYNCHRONIZE,
        System::Threading::{OpenProcess, WaitForSingleObject},
    };

    let Some(parent_process_id) = std::env::args().find_map(|argument| {
        argument
            .strip_prefix(RESTART_AS_ADMIN_ARG_PREFIX)
            .and_then(|value| value.parse::<u32>().ok())
    }) else {
        return;
    };

    let process = unsafe { OpenProcess(SYNCHRONIZE, 0, parent_process_id) };
    if process.is_null() {
        return;
    }

    unsafe {
        WaitForSingleObject(process, RESTART_PARENT_WAIT_MS);
        CloseHandle(process);
    }
}

#[cfg(not(windows))]
pub(crate) fn wait_for_restart_parent() {}

// 以管理员身份重新启动 Power Paste，用于向管理员窗口自动粘贴。
#[tauri::command]
pub(crate) fn restart_as_admin(app: AppHandle) -> Result<(), AppError> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_HIDE};

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
        let arguments = format!("{RESTART_AS_ADMIN_ARG_PREFIX}{}", std::process::id())
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();

        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                executable.as_ptr(),
                arguments.as_ptr(),
                std::ptr::null(),
                SW_HIDE,
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
