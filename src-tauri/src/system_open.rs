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
        return reveal_linux_path_with_fallback(path);
    }

    #[allow(unreachable_code)]
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_linux_open_path_with_fallback(target: &Path) -> Result<()> {
    spawn_linux_open_candidates(target.as_os_str(), "path")
}

// 一条定位文件的候选命令；D-Bus 客户端会立即退出，必须等待退出码才能判断
// 桌面文件管理器是否实现了该接口，文件管理器本体则只需成功拉起。
#[cfg(all(unix, not(target_os = "macos")))]
struct RevealCandidate {
    program: String,
    args: Vec<String>,
    wait_for_status: bool,
}

// 在文件管理器中选中指定文件：优先使用桌面标准的 FileManager1 接口（指向默认
// 文件管理器），其次按文件管理器自身的 --select 参数，最后回退到打开所在目录。
#[cfg(all(unix, not(target_os = "macos")))]
fn reveal_linux_path_with_fallback(path: &Path) -> Result<()> {
    if let Some(uri) = file_uri_from_path(path) {
        for candidate in linux_reveal_candidates(&uri, path) {
            if spawn_reveal_candidate(&candidate) {
                return Ok(());
            }
        }
    }

    let parent = path.parent().unwrap_or(path);
    spawn_linux_open_path_with_fallback(parent)
}

// 按优先级列出定位候选：D-Bus 接口 → nautilus / dolphin 的 --select。
#[cfg(all(unix, not(target_os = "macos")))]
fn linux_reveal_candidates(uri: &str, path: &Path) -> Vec<RevealCandidate> {
    let path_arg = path.to_string_lossy().to_string();
    vec![
        RevealCandidate {
            program: "gdbus".into(),
            args: vec![
                "call".into(),
                "--session".into(),
                "--dest".into(),
                "org.freedesktop.FileManager1".into(),
                "--object-path".into(),
                "/org/freedesktop/FileManager1".into(),
                "--method".into(),
                "org.freedesktop.FileManager1.ShowItems".into(),
                format!("['{uri}']"),
                "''".into(),
            ],
            wait_for_status: true,
        },
        RevealCandidate {
            program: "dbus-send".into(),
            args: vec![
                "--session".into(),
                "--dest=org.freedesktop.FileManager1".into(),
                "/org/freedesktop/FileManager1".into(),
                "org.freedesktop.FileManager1.ShowItems".into(),
                format!("array:string:{uri}"),
                "string:".into(),
            ],
            wait_for_status: true,
        },
        RevealCandidate {
            program: "nautilus".into(),
            args: vec!["--select".into(), path_arg.clone()],
            wait_for_status: false,
        },
        RevealCandidate {
            program: "dolphin".into(),
            args: vec!["--select".into(), path_arg],
            wait_for_status: false,
        },
    ]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_reveal_candidate(candidate: &RevealCandidate) -> bool {
    let mut command = std::process::Command::new(&candidate.program);
    command.args(&candidate.args);

    if candidate.wait_for_status {
        return matches!(command.output(), Ok(output) if output.status.success());
    }

    command.spawn().is_ok()
}

// 把绝对路径转换成百分号编码的 file:// URI，供 D-Bus 接口使用；
// 非绝对路径返回 None（对端无法定位）。
#[cfg(all(unix, not(target_os = "macos")))]
fn file_uri_from_path(path: &Path) -> Option<String> {
    if !path.is_absolute() {
        return None;
    }

    let text = path.to_str()?;
    let mut uri = String::from("file://");
    for byte in text.as_bytes() {
        let character = *byte as char;
        if character.is_ascii_alphanumeric()
            || matches!(character, '/' | '-' | '_' | '.' | '~')
        {
            uri.push(character);
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    Some(uri)
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
        Err(error.context(format!(
            "failed to open linux {target_kind} with desktop opener"
        )))
    } else {
        anyhow::bail!("failed to find a desktop opener for linux {target_kind}")
    }
}

#[cfg(all(test, unix, not(target_os = "macos")))]
mod tests {
    use std::path::Path;

    use super::{file_uri_from_path, linux_reveal_candidates};

    #[test]
    fn builds_percent_encoded_file_uris() {
        assert_eq!(
            file_uri_from_path(Path::new("/tmp/photo.png")).as_deref(),
            Some("file:///tmp/photo.png")
        );
        // 空格、中文与 # 都必须编码，否则 D-Bus 接口拿到的是无效 URI。
        assert_eq!(
            file_uri_from_path(Path::new("/tmp/我的 照片#1.png")).as_deref(),
            Some("file:///tmp/%E6%88%91%E7%9A%84%20%E7%85%A7%E7%89%87%231.png")
        );
        assert_eq!(file_uri_from_path(Path::new("relative.txt")), None);
    }

    #[test]
    fn prefers_the_desktop_file_manager_interface_before_fallbacks() {
        let candidates = linux_reveal_candidates("file:///tmp/a.txt", Path::new("/tmp/a.txt"));

        assert_eq!(candidates[0].program, "gdbus");
        assert!(candidates[0].wait_for_status);
        assert_eq!(candidates[1].program, "dbus-send");
        assert!(candidates
            .iter()
            .any(|candidate| candidate.program == "nautilus" && !candidate.wait_for_status));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.program == "dolphin"));
    }
}
