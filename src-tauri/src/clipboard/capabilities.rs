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
        preferred_clipboard_backend: preferred_clipboard_backend(),
        clipboard_write_strategy: clipboard_write_strategy(),
        direct_paste_strategy: direct_paste_strategy(),
        mixed_replay_strategy: mixed_replay_strategy(),
    }
}

pub(crate) fn launch_on_startup_supported() -> bool {
    cfg!(windows) || cfg!(target_os = "macos") || cfg!(target_os = "linux")
}

pub(crate) fn direct_paste_supported() -> bool {
    if cfg!(windows) || cfg!(target_os = "macos") {
        return true;
    }

    #[cfg(target_os = "linux")]
    {
        return linux_session_backend() == "x11" && linux_x11_tooling_available();
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
            if linux_session_backend() == "wayland" {
                return "linux_wayland_unsupported";
            }
            if !linux_x11_tooling_available() {
                return "linux_x11_tools_missing";
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
        "simulated-xdotool-shortcut"
    } else if cfg!(target_os = "linux") {
        #[cfg(target_os = "linux")]
        {
            if linux_session_backend() == "wayland" {
                return "wayland-unsupported";
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
        .flat_map(std::env::split_paths)
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

        if cfg!(windows) || cfg!(target_os = "macos") {
            assert!(capabilities.supports_direct_paste);
        } else if cfg!(target_os = "linux") {
            let expected =
                super::linux_session_backend() == "x11" && super::linux_x11_tooling_available();
            assert_eq!(capabilities.supports_direct_paste, expected);
        } else {
            assert!(!capabilities.supports_direct_paste);
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
        if cfg!(target_os = "linux") {
            let supports_direct_paste =
                super::linux_session_backend() == "x11" && super::linux_x11_tooling_available();
            if supports_direct_paste {
                assert_eq!(
                    direct_paste_unavailable_reason(),
                    "unsupported_direct_paste"
                );
            } else {
                assert!(matches!(
                    direct_paste_unavailable_reason(),
                    "linux_wayland_unsupported" | "linux_x11_tools_missing"
                ));
            }
        } else {
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
}
