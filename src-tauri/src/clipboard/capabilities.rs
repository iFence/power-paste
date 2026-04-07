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
        supports_launch_on_startup: true,
        preferred_clipboard_backend: preferred_clipboard_backend(),
        clipboard_write_strategy: clipboard_write_strategy(),
        direct_paste_strategy: direct_paste_strategy(),
        mixed_replay_strategy: mixed_replay_strategy(),
    }
}

pub(crate) fn preferred_clipboard_backend() -> &'static str {
    if cfg!(windows) || cfg!(target_os = "macos") {
        "plugin+native-fallback"
    } else {
        "plugin-only"
    }
}

fn clipboard_write_strategy() -> &'static str {
    if cfg!(windows) || cfg!(target_os = "macos") {
        "plugin-first-with-native-fallback"
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
        "native-fallback-write-only"
    } else {
        "unsupported"
    }
}
