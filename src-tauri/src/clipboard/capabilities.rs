use crate::models::PlatformCapabilities;

pub(crate) fn platform_capabilities() -> PlatformCapabilities {
    let supports_direct_paste = direct_paste_supported();
    let supports_mixed_replay = mixed_replay_supported();

    PlatformCapabilities {
        platform: std::env::consts::OS.to_string(),
        supports_clipboard_read: true,
        supports_clipboard_watch: true,
        supports_text_write: true,
        supports_html_write: true,
        supports_image_write: true,
        supports_direct_paste,
        supports_mixed_replay,
        supports_launch_on_startup: launch_on_startup_supported(),
        supports_hardware_acceleration_toggle: hardware_acceleration_toggle_supported(),
        preferred_clipboard_backend: preferred_clipboard_backend(),
        clipboard_write_strategy: clipboard_write_strategy(),
        direct_paste_strategy: direct_paste_strategy(),
        mixed_replay_strategy: mixed_replay_strategy(),
    }
}

pub(crate) fn launch_on_startup_supported() -> bool {
    cfg!(windows) || cfg!(target_os = "macos") || cfg!(target_os = "linux")
}

pub(crate) fn hardware_acceleration_toggle_supported() -> bool {
    cfg!(windows) || cfg!(target_os = "macos")
}

pub(crate) fn direct_paste_supported() -> bool {
    if cfg!(windows) || cfg!(target_os = "macos") {
        return true;
    }

    #[cfg(target_os = "linux")]
    {
        return linux_direct_paste_supported();
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

pub(crate) fn direct_paste_unavailable_reason() -> &'static str {
    if cfg!(target_os = "linux") {
        #[cfg(target_os = "linux")]
        {
            match linux_direct_paste_backend() {
                "wayland" if !linux_wayland_input_available() => {
                    return "linux_wayland_tools_missing";
                }
                "x11" if !linux_x11_tooling_available() => {
                    return "linux_x11_tools_missing";
                }
                _ => {}
            }
        }
    }

    "unsupported_direct_paste"
}

pub(crate) fn preferred_clipboard_backend() -> &'static str {
    if cfg!(windows) {
        "plugin+native-fallback"
    } else if cfg!(target_os = "macos") {
        "plugin-preferred"
    } else {
        "plugin-only"
    }
}

fn clipboard_write_strategy() -> &'static str {
    if cfg!(windows) {
        "plugin-first-with-native-fallback"
    } else if cfg!(target_os = "macos") {
        "plugin-first-with-mixed-degradation"
    } else {
        "plugin-only"
    }
}

fn direct_paste_strategy() -> &'static str {
    if cfg!(windows) || cfg!(target_os = "macos") {
        "simulated-native-shortcut"
    } else if direct_paste_supported() {
        #[cfg(target_os = "linux")]
        {
            if linux_direct_paste_backend() == "wayland" {
                if linux_wayland_tooling_available() {
                    return "simulated-wtype-shortcut";
                }
                if linux_ydotool_available() {
                    return "simulated-ydotool-shortcut";
                }
                return "simulated-portal-shortcut";
            }
        }
        "simulated-xdotool-shortcut"
    } else if cfg!(target_os = "linux") {
        #[cfg(target_os = "linux")]
        {
            if linux_session_backend() == "wayland" {
                return "wayland-wtype-required";
            }
        }
        "x11-tooling-required"
    } else {
        "unsupported"
    }
}

fn mixed_replay_strategy() -> &'static str {
    if cfg!(windows) {
        "target-aware-segmented-replay"
    } else if cfg!(target_os = "macos") {
        "plugin-degraded-single-payload"
    } else if direct_paste_supported() {
        "plugin-degraded-single-payload"
    } else {
        "unsupported"
    }
}

fn mixed_replay_supported() -> bool {
    cfg!(windows)
}

#[cfg(target_os = "linux")]
pub(crate) fn linux_session_backend() -> &'static str {
    linux_session_backend_with(
        std::env::var("DISPLAY").ok(),
        std::env::var("WAYLAND_DISPLAY").ok(),
        std::env::var("XDG_SESSION_TYPE").ok(),
    )
}

#[cfg(target_os = "linux")]
pub(crate) fn linux_x11_tooling_available() -> bool {
    binary_in_path("xdotool")
}

#[cfg(target_os = "linux")]
pub(crate) fn linux_wayland_tooling_available() -> bool {
    binary_in_path("wtype")
}

/// ydotool 走内核 uinput，任何合成器都能用，但需要用户自行配置守护进程与权限。
#[cfg(target_os = "linux")]
pub(crate) fn linux_ydotool_available() -> bool {
    binary_in_path("ydotool")
}

/// 桌面是否为 Wayland 会话提供了远程输入注入通道。
///
/// GNOME / KDE 的合成器不实现虚拟键盘协议（wtype 因此不可用），改为让桌面通过
/// `org.freedesktop.portal.RemoteDesktop` 授权按键注入；门户后端声明了该接口时
/// 才算可用（精简的 wlroots 会话通常只提供截图 / 录屏）。
#[cfg(target_os = "linux")]
pub(crate) fn linux_remote_input_portal_available() -> bool {
    remote_input_portal_available_in(&portal_data_roots())
}

const REMOTE_DESKTOP_IMPL_INTERFACE: &str = "org.freedesktop.impl.portal.RemoteDesktop";

#[cfg(target_os = "linux")]
fn remote_input_portal_available_in(roots: &[std::path::PathBuf]) -> bool {
    roots.iter().any(|root| {
        std::fs::read_dir(root.join("xdg-desktop-portal/portals"))
            .into_iter()
            .flatten()
            .flatten()
            .any(|entry| {
                std::fs::read_to_string(entry.path())
                    .map(|content| content.contains(REMOTE_DESKTOP_IMPL_INTERFACE))
                    .unwrap_or(false)
            })
    })
}

/// XDG 约定的数据目录：`$XDG_DATA_HOME` 与 `$XDG_DATA_DIRS`，门户后端描述文件
/// 位于其下的 `xdg-desktop-portal/portals`。
#[cfg(target_os = "linux")]
fn portal_data_roots() -> Vec<std::path::PathBuf> {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".local/share"))
        });
    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .into_iter()
        .flat_map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .collect::<Vec<_>>();

    let mut roots = Vec::new();
    if let Some(data_home) = data_home {
        roots.push(data_home);
    }
    if data_dirs.is_empty() {
        roots.push(std::path::PathBuf::from("/usr/local/share"));
        roots.push(std::path::PathBuf::from("/usr/share"));
    } else {
        roots.extend(data_dirs);
    }

    roots
}

/// Wayland 下是否存在任一种按键注入手段。
#[cfg(target_os = "linux")]
fn linux_wayland_input_available() -> bool {
    linux_wayland_tooling_available()
        || linux_ydotool_available()
        || linux_remote_input_portal_available()
}

#[cfg(target_os = "linux")]
fn linux_direct_paste_supported() -> bool {
    linux_direct_paste_supported_with(
        linux_direct_paste_backend(),
        linux_x11_tooling_available(),
        linux_wayland_input_available(),
    )
}

#[cfg(target_os = "linux")]
fn linux_direct_paste_supported_with(
    direct_paste_backend: &str,
    x11_tooling_available: bool,
    wayland_tooling_available: bool,
) -> bool {
    match direct_paste_backend {
        "x11" => x11_tooling_available,
        "wayland" => wayland_tooling_available,
        _ => false,
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn linux_direct_paste_backend() -> &'static str {
    linux_direct_paste_backend_with(
        linux_session_backend(),
        linux_x11_tooling_available(),
        linux_wayland_input_available(),
    )
}

#[cfg(target_os = "linux")]
fn linux_direct_paste_backend_with(
    session_backend: &str,
    x11_tooling_available: bool,
    wayland_tooling_available: bool,
) -> &'static str {
    match session_backend {
        "wayland" => "wayland",
        "x11" => "x11",
        _ if x11_tooling_available => "x11",
        _ if wayland_tooling_available => "wayland",
        _ => "x11",
    }
}

#[cfg(target_os = "linux")]
fn linux_session_backend_with(
    display: Option<String>,
    wayland_display: Option<String>,
    session_type: Option<String>,
) -> &'static str {
    let session_type = session_type.unwrap_or_default().to_lowercase();
    let has_display = display
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let has_wayland_display = wayland_display
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    if session_type == "wayland" || has_wayland_display {
        "wayland"
    } else if session_type == "x11" || has_display {
        "x11"
    } else {
        "unknown"
    }
}

#[cfg(target_os = "linux")]
fn binary_in_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|unparsed| std::env::split_paths(&unparsed).collect::<Vec<_>>())
        .map(|path| path.join(name))
        .any(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::{
        direct_paste_unavailable_reason, launch_on_startup_supported, platform_capabilities,
    };

    #[test]
    fn direct_paste_support_matches_platform_policy() {
        let capabilities = platform_capabilities();

        #[cfg(any(windows, target_os = "macos"))]
        {
            assert!(capabilities.supports_direct_paste);
        }

        #[cfg(target_os = "linux")]
        {
            let expected = super::linux_direct_paste_supported();

            assert_eq!(capabilities.supports_direct_paste, expected);
        }

        #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
        {
            let expected = false;

            assert_eq!(capabilities.supports_direct_paste, expected);
        }
    }

    #[test]
    fn launch_on_startup_support_matches_platform_policy() {
        let capabilities = platform_capabilities();

        assert_eq!(
            capabilities.supports_launch_on_startup,
            launch_on_startup_supported()
        );
    }

    #[test]
    fn hardware_acceleration_toggle_support_matches_platform_policy() {
        let capabilities = platform_capabilities();

        assert_eq!(
            capabilities.supports_hardware_acceleration_toggle,
            super::hardware_acceleration_toggle_supported()
        );
    }

    #[test]
    fn mixed_replay_support_is_windows_only() {
        let capabilities = platform_capabilities();

        if cfg!(windows) {
            assert!(capabilities.supports_mixed_replay);
        } else {
            assert!(!capabilities.supports_mixed_replay);
        }
    }

    #[test]
    fn direct_paste_reason_matches_linux_policy() {
        #[cfg(target_os = "linux")]
        {
            let expected = super::linux_direct_paste_supported();
            if expected {
                assert_eq!(
                    direct_paste_unavailable_reason(),
                    "unsupported_direct_paste"
                );
            } else {
                assert!(matches!(
                    direct_paste_unavailable_reason(),
                    "linux_wayland_tools_missing" | "linux_x11_tools_missing"
                ));
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            assert_eq!(
                direct_paste_unavailable_reason(),
                "unsupported_direct_paste"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_session_backend_prefers_wayland_when_present() {
        assert_eq!(
            super::linux_session_backend_with(
                Some(":0".into()),
                Some("wayland-0".into()),
                Some("x11".into())
            ),
            "wayland"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_session_backend_detects_x11_from_display() {
        assert_eq!(
            super::linux_session_backend_with(Some(":0".into()), None, None),
            "x11"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_direct_paste_supports_wayland_when_wtype_is_available() {
        assert!(super::linux_direct_paste_supported_with(
            "wayland", false, true
        ));
        assert!(!super::linux_direct_paste_supported_with(
            "wayland", false, false
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_direct_paste_backend_falls_back_to_available_tooling_when_session_is_unknown() {
        assert_eq!(
            super::linux_direct_paste_backend_with("unknown", true, false),
            "x11"
        );
        assert_eq!(
            super::linux_direct_paste_backend_with("unknown", false, true),
            "wayland"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn remote_input_portal_is_detected_from_backend_descriptions() {
        let root = std::env::temp_dir().join(format!(
            "power-paste-portals-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let portals_dir = root.join("xdg-desktop-portal/portals");
        std::fs::create_dir_all(&portals_dir).expect("create portals dir");
        std::fs::write(
            portals_dir.join("wlr.portal"),
            "[portal]\nDBusName=org.freedesktop.impl.portal.desktop.wlr\nInterfaces=org.freedesktop.impl.portal.ScreenCast;\n",
        )
        .expect("write wlr portal");

        assert!(!super::remote_input_portal_available_in(&[root.clone()]));

        // GNOME / KDE 的后端会声明 RemoteDesktop，据此判断 Wayland 下仍可注入按键。
        std::fs::write(
            portals_dir.join("gnome.portal"),
            "[portal]\nDBusName=org.freedesktop.impl.portal.desktop.gnome\nInterfaces=org.freedesktop.impl.portal.RemoteDesktop;\n",
        )
        .expect("write gnome portal");

        assert!(super::remote_input_portal_available_in(&[root.clone()]));
        std::fs::remove_dir_all(&root).expect("cleanup portals dir");
    }
}
