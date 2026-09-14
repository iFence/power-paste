//! Linux 下的剪贴板来源应用识别与图标解析。
//!
//! 只要应用能在 X 服务器里看到窗口，这里就能定位来源：优先取剪贴板的 selection
//! owner（最贴近「内容从哪来」），X11 会话下再退化到 `_NET_ACTIVE_WINDOW`。拿到
//! `WM_CLASS` 与 `_NET_WM_PID` 后顺着进程、`.desktop` 文件与图标主题取出应用名和
//! 图标内容。
//!
//! 纯 Wayland 客户端不会出现在 X 服务器里（GNOME 的 `org.gnome.Shell.Introspect`
//! 对普通应用返回 AccessDenied，Wayland 协议本身也不携带剪贴板来源），因此这类来源
//! 仍然识别不了，界面继续显示占位图标。

use std::{
    fs,
    path::{Path, PathBuf},
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use x11rb::{
    connection::Connection,
    protocol::xproto::{Atom, AtomEnum, ConnectionExt, Window},
    rust_connection::RustConnection,
};

use crate::models::ForegroundAppResult;

/// 图标主题查找顺序：先发行版默认的高清主题，再退化到常见第三方主题。
const ICON_THEMES: &[&str] = &[
    "hicolor",
    "Adwaita",
    "breeze",
    "breeze-dark",
    "Yaru",
    "Papirus",
    "elementary",
];

/// 图标尺寸查找顺序：大图优先，`scalable`（SVG）放在大位图之后、小位图之前。
const ICON_SIZES: &[&str] = &[
    "512x512",
    "256x256",
    "192x192",
    "128x128",
    "scalable",
    "96x96",
    "64x64",
    "48x48",
    "32x32",
];

/// 识别当前剪贴板来源应用；拿不到窗口归属时返回 None。
pub(crate) fn capture_foreground_app() -> Option<ForegroundAppResult> {
    if !has_x_display() {
        return None;
    }

    let (connection, screen_num) = x11rb::connect(None).ok()?;
    let root = connection.setup().roots.get(screen_num)?.root;

    let identity = clipboard_owner(&connection)
        .and_then(|window| window_identity(&connection, root, window))
        .or_else(|| {
            // Wayland 会话里剪贴板 owner 是合成器的桥接窗口（没有任何属性），此时
            // 活动窗口并不代表内容来源，只有 X11 会话才用它兜底。
            if crate::clipboard::linux_session_backend() != "x11" {
                return None;
            }
            active_window(&connection, root)
                .and_then(|window| window_identity(&connection, root, window))
        })?;

    build_app_result(identity)
}

/// 窗口归属：`WM_CLASS` 的 res_class 与 `_NET_WM_PID`，两者至少有一个才有意义。
fn window_identity(
    connection: &RustConnection,
    root: Window,
    window: Window,
) -> Option<WindowIdentity> {
    // selection owner 常是工具包的内部窗口（没有 WM_CLASS），所以按窗口层级向上找。
    let resource_class =
        window_attribute(connection, root, window, window_class).unwrap_or_default();
    let pid = window_attribute(connection, root, window, window_pid);
    // 有些工具包把 `WM_CLASS` 只放在顶层窗口上，而 selection owner 是内部窗口；
    // 这时按 PID 在 `_NET_CLIENT_LIST` 里找到同一个进程的顶层窗口再读一次。
    let resource_class = if resource_class.is_empty() {
        pid.and_then(|pid| client_window_for_pid(connection, root, pid))
            .and_then(|toplevel| window_attribute(connection, root, toplevel, window_class))
            .unwrap_or(resource_class)
    } else {
        resource_class
    };

    if resource_class.is_empty() && pid.is_none() {
        return None;
    }

    Some(WindowIdentity {
        resource_class,
        pid,
    })
}

/// 在 `_NET_CLIENT_LIST` 里按 PID 找同一个进程的顶层窗口。
fn client_window_for_pid(connection: &RustConnection, root: Window, pid: u32) -> Option<Window> {
    let atom = intern_atom(connection, b"_NET_CLIENT_LIST")?;
    let reply = connection
        .get_property(false, root, atom, AtomEnum::WINDOW, 0, 1024)
        .ok()?
        .reply()
        .ok()?;

    let toplevel = reply
        .value32()?
        .find(|window| window_pid(connection, *window) == Some(pid));
    toplevel
}

/// 从窗口本身向上逐级查找属性；工具包会把 `WM_CLASS` / `_NET_WM_PID` 放在顶层窗口上。
fn window_attribute<T>(
    connection: &RustConnection,
    root: Window,
    window: Window,
    read: fn(&RustConnection, Window) -> Option<T>,
) -> Option<T> {
    let mut current = window;
    for _ in 0..16 {
        if let Some(value) = read(connection, current) {
            return Some(value);
        }

        let parent = connection.query_tree(current).ok()?.reply().ok()?.parent;
        if parent == x11rb::NONE || parent == root {
            break;
        }
        current = parent;
    }

    None
}

fn build_app_result(identity: WindowIdentity) -> Option<ForegroundAppResult> {
    let process_name = identity
        .pid
        .and_then(process_name_of)
        .unwrap_or_default();
    let app_path = identity.pid.and_then(process_exe_of);
    let desktop = find_desktop_entry(
        &application_dirs(),
        &identity.resource_class,
        &process_name,
        app_path.as_deref(),
    );

    let display_name = desktop
        .as_ref()
        .and_then(|entry| entry.name.clone())
        .unwrap_or_default();
    let icon = desktop
        .as_ref()
        .and_then(|entry| entry.icon.as_deref())
        .and_then(|name| resolve_icon(&icon_roots(), name));

    if display_name.is_empty() && process_name.is_empty() {
        return None;
    }

    let (icon_base64, icon_media_type) = match icon {
        Some((bytes, media_type)) => (Some(BASE64.encode(bytes)), Some(media_type.to_string())),
        None => (None, None),
    };

    Some(ForegroundAppResult {
        process_name,
        display_name,
        icon_base64,
        icon_media_type,
        app_path,
        bundle_id: None,
    })
}

struct WindowIdentity {
    resource_class: String,
    pid: Option<u32>,
}

/// 读取窗口的 `WM_CLASS`：协议里是 `res_name\0res_class\0`，取 res_class。
fn window_class(connection: &RustConnection, window: Window) -> Option<String> {
    let reply = connection
        .get_property(
            false,
            window,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            0,
            1024,
        )
        .ok()?
        .reply()
        .ok()?;

    let mut parts = reply
        .value
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty());
    let resource_name = parts.next()?;
    let resource_class = parts.next().unwrap_or(resource_name);

    String::from_utf8(resource_class.to_vec())
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn window_pid(connection: &RustConnection, window: Window) -> Option<u32> {
    let atom = intern_atom(connection, b"_NET_WM_PID")?;
    let reply = connection
        .get_property(false, window, atom, AtomEnum::CARDINAL, 0, 1)
        .ok()?
        .reply()
        .ok()?;
    let pid = reply.value32()?.next();
    pid
}

fn clipboard_owner(connection: &RustConnection) -> Option<Window> {
    let atom = intern_atom(connection, b"CLIPBOARD")?;
    let owner = connection
        .get_selection_owner(atom)
        .ok()?
        .reply()
        .ok()?
        .owner;
    (owner != x11rb::NONE).then_some(owner)
}

fn active_window(connection: &RustConnection, root: Window) -> Option<Window> {
    let atom = intern_atom(connection, b"_NET_ACTIVE_WINDOW")?;
    let reply = connection
        .get_property(false, root, atom, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?;
    let active = reply
        .value32()?
        .next()
        .filter(|window| *window != x11rb::NONE);
    active
}

fn intern_atom(connection: &RustConnection, name: &[u8]) -> Option<Atom> {
    let atom = connection
        .intern_atom(false, name)
        .ok()?
        .reply()
        .ok()?
        .atom;
    (atom != x11rb::NONE).then_some(atom)
}

fn has_x_display() -> bool {
    std::env::var("DISPLAY")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn process_name_of(pid: u32) -> Option<String> {
    let name = fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    let trimmed = name.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn process_exe_of(pid: u32) -> Option<String> {
    let path = fs::read_link(format!("/proc/{pid}/exe")).ok()?;
    Some(path.to_string_lossy().to_string())
}

/// 桌面项里我们用得到的字段。
struct DesktopEntry {
    name: Option<String>,
    icon: Option<String>,
    startup_wm_class: Option<String>,
}

/// 按 `WM_CLASS`、进程名、可执行文件名依次匹配 `.desktop`；都不中时再扫描
/// `StartupWMClass`（部分应用把窗口类写成了自己的桌面项 id 之外的值）。
fn find_desktop_entry(
    dirs: &[PathBuf],
    resource_class: &str,
    process_name: &str,
    app_path: Option<&str>,
) -> Option<DesktopEntry> {
    let mut candidates: Vec<String> = Vec::new();
    for value in [resource_class, process_name] {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        for candidate in [trimmed.to_string(), trimmed.to_ascii_lowercase()] {
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
    }
    if let Some(stem) = app_path
        .and_then(|path| Path::new(path).file_stem())
        .and_then(|value| value.to_str())
    {
        if !stem.is_empty() && !candidates.iter().any(|value| value == stem) {
            candidates.push(stem.to_string());
        }
    }

    for candidate in &candidates {
        for dir in dirs {
            let path = dir.join(format!("{candidate}.desktop"));
            if let Some(entry) = parse_desktop_entry(&path) {
                return Some(entry);
            }
        }
    }

    let target = resource_class.trim().to_ascii_lowercase();
    if target.is_empty() {
        return None;
    }
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("desktop") {
                continue;
            }
            let Some(desktop) = parse_desktop_entry(&path) else {
                continue;
            };
            let matches = desktop
                .startup_wm_class
                .as_deref()
                .map(|value| value.trim().eq_ignore_ascii_case(&target))
                .unwrap_or(false);
            if matches {
                return Some(desktop);
            }
        }
    }

    None
}

/// 只解析 `[Desktop Entry]` 段里的 Name / Icon / StartupWMClass。
fn parse_desktop_entry(path: &Path) -> Option<DesktopEntry> {
    let content = fs::read_to_string(path).ok()?;
    let mut entry = DesktopEntry {
        name: None,
        icon: None,
        startup_wm_class: None,
    };
    let mut in_main_section = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_main_section = line == "[Desktop Entry]";
            continue;
        }
        if !in_main_section {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }

        match key.trim() {
            "Name" => entry.name = Some(value.to_string()),
            "Icon" => entry.icon = Some(value.to_string()),
            "StartupWMClass" => entry.startup_wm_class = Some(value.to_string()),
            _ => {}
        }
    }

    (entry.name.is_some() || entry.icon.is_some()).then_some(entry)
}

/// 解析 `Icon=`：绝对路径直接读取，图标名则在图标主题目录里查找。
fn resolve_icon(roots: &[PathBuf], name: &str) -> Option<(Vec<u8>, &'static str)> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    if name.contains('/') {
        return read_icon_file(Path::new(name));
    }

    for root in roots {
        for theme in ICON_THEMES {
            for size in ICON_SIZES {
                for extension in ["png", "svg"] {
                    let path = root
                        .join(theme)
                        .join(size)
                        .join("apps")
                        .join(format!("{name}.{extension}"));
                    if let Some(icon) = read_icon_file(&path) {
                        return Some(icon);
                    }
                }
            }
        }
    }

    None
}

fn read_icon_file(path: &Path) -> Option<(Vec<u8>, &'static str)> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let media_type = match extension.as_str() {
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => return None,
    };

    let bytes = fs::read(path).ok()?;
    (!bytes.is_empty()).then_some((bytes, media_type))
}

fn application_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(data_home) = data_home() {
        dirs.push(data_home.join("applications"));
    }
    for dir in data_dirs() {
        dirs.push(dir.join("applications"));
    }
    dirs
}

fn icon_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(data_home) = data_home() {
        roots.push(data_home.join("icons"));
    }
    for dir in data_dirs() {
        roots.push(dir.join("icons"));
    }
    roots
}

fn data_home() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os("XDG_DATA_HOME") {
        let path = PathBuf::from(value);
        if !path.as_os_str().is_empty() {
            return Some(path);
        }
    }

    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
}

/// XDG 数据目录 + Flatpak 导出目录（部分发行版不把导出目录放进 XDG_DATA_DIRS）。
fn data_dirs() -> Vec<PathBuf> {
    let configured = std::env::var("XDG_DATA_DIRS").unwrap_or_default();
    let entries: Vec<&str> = if configured.trim().is_empty() {
        vec!["/usr/local/share", "/usr/share"]
    } else {
        configured.split(':').collect()
    };

    let mut dirs: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        let path = PathBuf::from(trimmed);
        if !dirs.contains(&path) {
            dirs.push(path);
        }
    }

    let mut flatpak_dirs = vec![PathBuf::from("/var/lib/flatpak/exports/share")];
    if let Some(home) = std::env::var_os("HOME") {
        flatpak_dirs.push(PathBuf::from(home).join(".local/share/flatpak/exports/share"));
    }
    for path in flatpak_dirs {
        if !dirs.contains(&path) {
            dirs.push(path);
        }
    }

    dirs
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_dir(tag: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("power-paste-linux-app-{tag}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn parses_desktop_entry_name_and_icon() {
        let dir = temp_dir("desktop");
        let path = dir.join("org.example.App.desktop");
        fs::write(
            &path,
            "[Desktop Entry]\nType=Application\nName=Example\nIcon=org.example.App\nStartupWMClass=Example\n\n[Desktop Action]\nName=Action\n",
        )
        .expect("write desktop entry");

        let entry = parse_desktop_entry(&path).expect("parsed entry");
        assert_eq!(entry.name.as_deref(), Some("Example"));
        assert_eq!(entry.icon.as_deref(), Some("org.example.App"));
        assert_eq!(entry.startup_wm_class.as_deref(), Some("Example"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn finds_desktop_entry_by_wm_class() {
        let dir = temp_dir("applications");
        fs::write(
            dir.join("org.example.App.desktop"),
            "[Desktop Entry]\nName=Example\nIcon=org.example.App\n",
        )
        .expect("write desktop entry");

        let entry = find_desktop_entry(&[dir.clone()], "org.example.App", "", None)
            .expect("entry found by resource class");
        assert_eq!(entry.name.as_deref(), Some("Example"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolves_icon_from_theme_directory() {
        let root = temp_dir("icons");
        let apps_dir = root.join("hicolor").join("128x128").join("apps");
        fs::create_dir_all(&apps_dir).expect("create icon dir");
        fs::write(apps_dir.join("org.example.App.png"), b"png-bytes").expect("write icon");

        let (bytes, media_type) =
            resolve_icon(&[root.clone()], "org.example.App").expect("icon resolved");
        assert_eq!(bytes, b"png-bytes");
        assert_eq!(media_type, "image/png");

        assert!(resolve_icon(&[root.clone()], "missing.Icon").is_none());
        let _ = fs::remove_dir_all(root);
    }
}
