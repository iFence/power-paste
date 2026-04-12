mod backend;
mod capabilities;
mod capture_router;
mod native_writer;
mod payload;
mod plugin_reader;
mod plugin_writer;

use anyhow::Result;
use tauri::AppHandle;

use crate::{models::StoredClipboardItem, paste_target::TargetProfile};

use self::payload::{payload_for_item, ClipboardPayload};

pub(crate) use capabilities::direct_paste_unavailable_reason;
pub(crate) use capabilities::{launch_on_startup_supported, platform_capabilities};
#[cfg(target_os = "linux")]
pub(crate) use capabilities::{linux_session_backend, linux_x11_tooling_available};
pub(crate) use capture_router::capture_clipboard;
#[cfg(windows)]
pub(crate) use native_writer::write_image_to_clipboard;
#[cfg(windows)]
pub(crate) use plugin_writer::{
    write_image as write_image_with_plugin, write_text as write_text_with_plugin,
};

pub(crate) fn write_item_to_clipboard_with_profile(
    app: &AppHandle,
    item: &StoredClipboardItem,
    profile: TargetProfile,
) -> Result<ClipboardPayload> {
    let payload = payload_for_item(item);

    match backend::preferred_backend_for_payload(&payload) {
        crate::models::ClipboardBackend::Plugin => {
            let payload = degrade_plugin_only_payload(payload);
            plugin_writer::write_payload(app, &payload)
        }
        crate::models::ClipboardBackend::NativeFallback => {
            native_writer::write_payload(item, profile, &payload)
                .map(|_| payload.clone())
                .or_else(|_| plugin_writer::write_payload(app, &plugin_fallback_payload(payload)))
        }
    }
}

pub(crate) fn wait_for_clipboard_payload(
    app: &AppHandle,
    payload: &ClipboardPayload,
) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let _ = app;
        let delay_ms = match payload {
            ClipboardPayload::Image { .. } => 120,
            ClipboardPayload::RichText { .. } | ClipboardPayload::Html { .. } => 80,
            _ => 40,
        };
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        let _ = payload;
        Ok(())
    }
}

fn plugin_fallback_payload(payload: ClipboardPayload) -> ClipboardPayload {
    match payload {
        ClipboardPayload::Html { text, html } => {
            if let Some(text) = text {
                ClipboardPayload::Text { text }
            } else {
                ClipboardPayload::Html { text: None, html }
            }
        }
        ClipboardPayload::RichText { text, html, rtf } => {
            if let Some(text) = text {
                ClipboardPayload::Text { text }
            } else if let Some(html) = html {
                ClipboardPayload::Html { text: None, html }
            } else if let Some(rtf) = rtf {
                ClipboardPayload::RichText {
                    text: None,
                    html: None,
                    rtf: Some(rtf),
                }
            } else {
                ClipboardPayload::Empty
            }
        }
        ClipboardPayload::Mixed {
            text,
            html,
            png_bytes,
        } => {
            if let Some(text) = text {
                ClipboardPayload::Text { text }
            } else if let Some(html) = html {
                ClipboardPayload::Html { text: None, html }
            } else {
                ClipboardPayload::Image { png_bytes }
            }
        }
        other => other,
    }
}

fn degrade_plugin_only_payload(payload: ClipboardPayload) -> ClipboardPayload {
    if cfg!(any(windows, target_os = "macos")) {
        payload
    } else {
        match payload {
            ClipboardPayload::Mixed { .. } => plugin_fallback_payload(payload),
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::degrade_plugin_only_payload;
    use crate::clipboard::payload::ClipboardPayload;

    #[test]
    fn degrades_mixed_payload_to_single_payload_on_plugin_only_platforms() {
        let payload = ClipboardPayload::Mixed {
            text: Some("plain".into()),
            html: Some("<b>plain</b>".into()),
            png_bytes: vec![1, 2, 3],
        };

        let next = degrade_plugin_only_payload(payload);

        if cfg!(windows) || cfg!(target_os = "macos") {
            assert!(matches!(next, ClipboardPayload::Mixed { .. }));
        } else {
            assert!(matches!(next, ClipboardPayload::Text { .. }));
        }
    }

    #[test]
    fn degrades_image_only_mixed_payload_to_image_on_plugin_only_platforms() {
        let payload = ClipboardPayload::Mixed {
            text: None,
            html: None,
            png_bytes: vec![1, 2, 3],
        };

        let next = degrade_plugin_only_payload(payload);

        if cfg!(windows) || cfg!(target_os = "macos") {
            assert!(matches!(next, ClipboardPayload::Mixed { .. }));
        } else {
            assert!(matches!(next, ClipboardPayload::Image { .. }));
        }
    }
}
