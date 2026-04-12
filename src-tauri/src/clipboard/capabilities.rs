use crate::models::PlatformCapabilities;

pub(crate) fn platform_capabilities() -> PlatformCapabilities {
    PlatformCapabilities {
        platform: std::env::consts::OS.to_string(),
        supports_clipboard_read: true,
        supports_clipboard_watch: true,
        supports_text_write: true,
        supports_html_write: true,
        supports_image_write: true,
        supports_direct_paste: cfg!(windows) || cfg!(target_os = "macos"),
        supports_mixed_replay: cfg!(windows),
        supports_launch_on_startup: launch_on_startup_supported(),
        preferred_clipboard_backend: preferred_clipboard_backend(),
        clipboard_write_strategy: clipboard_write_strategy(),
        direct_paste_strategy: direct_paste_strategy(),
        mixed_replay_strategy: mixed_replay_strategy(),
    }
}

pub(crate) fn launch_on_startup_supported() -> bool {
    cfg!(windows) || cfg!(target_os = "macos")
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
    } else {
        "unsupported"
    }
}

fn mixed_replay_strategy() -> &'static str {
    if cfg!(windows) {
        "target-aware-segmented-replay"
    } else if cfg!(target_os = "macos") {
        "plugin-degraded-single-payload"
    } else {
        "unsupported"
    }
}

#[cfg(test)]
mod tests {
    use super::{launch_on_startup_supported, platform_capabilities};

    #[test]
    fn direct_paste_support_matches_platform_policy() {
        let capabilities = platform_capabilities();

        if cfg!(windows) || cfg!(target_os = "macos") {
            assert!(capabilities.supports_direct_paste);
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
}
