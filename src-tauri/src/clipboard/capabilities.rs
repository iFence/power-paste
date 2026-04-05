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
    }
}

pub(crate) fn preferred_clipboard_backend() -> &'static str {
    if cfg!(windows) || cfg!(target_os = "macos") {
        "plugin+native-fallback"
    } else {
        "plugin-only"
    }
}
