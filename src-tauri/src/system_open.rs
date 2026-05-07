use std::path::Path;

use anyhow::{Context, Result};

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::ShellExecuteW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

// 使用系统默认处理器打开网页链接。
pub(crate) fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let operation: Vec<u16> = "open\0".encode_utf16().collect();
        let target: Vec<u16> = format!("{url}\0").encode_utf16().collect();
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
            anyhow::bail!("failed to open external url: {url}");
        }

        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .context("failed to open external url")?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return spawn_linux_open_url_with_fallback(url);
    }

    #[allow(unreachable_code)]
    Ok(())
}

// 使用系统默认处理器打开本地文件。
pub(crate) fn open_path(path: &Path) -> Result<()> {
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
            anyhow::bail!("failed to open local file: {}", path.display());
        }

        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .context("failed to open local file")?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return spawn_linux_open_path_with_fallback(path);
    }

    #[allow(unreachable_code)]
    Ok(())
}

// 在系统文件管理器中显示目标文件；Linux 平台退化为打开其所在目录。
pub(crate) fn reveal_path(path: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", path.to_string_lossy()))
            .spawn()
            .context("failed to reveal local file")?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .context("failed to reveal local file")?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let parent = path.parent().unwrap_or(path);
        return spawn_linux_open_path_with_fallback(parent);
    }

    #[allow(unreachable_code)]
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_linux_open_path_with_fallback(target: &Path) -> Result<()> {
    spawn_linux_open_candidates(target.as_os_str(), "path")
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_linux_open_url_with_fallback(target: &str) -> Result<()> {
    spawn_linux_open_candidates(std::ffi::OsStr::new(target), "url")
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_linux_open_candidates(target: &std::ffi::OsStr, target_kind: &str) -> Result<()> {
    let candidates: [(&str, &[&str]); 5] = [
        ("xdg-open", &[]),
        ("gio", &["open"]),
        ("gnome-open", &[]),
        ("kde-open", &[]),
        ("kioclient", &["exec"]),
    ];
    let mut last_error = None;

    for (program, prefix_args) in candidates {
        let mut command = std::process::Command::new(program);
        command.args(prefix_args).arg(target);

        match command.spawn() {
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => last_error = Some(anyhow::Error::from(error)),
        }
    }

    if let Some(error) = last_error {
        Err(error.context(format!("failed to open linux {target_kind} with desktop opener")))
    } else {
        anyhow::bail!("failed to find a desktop opener for linux {target_kind}")
    }
}
