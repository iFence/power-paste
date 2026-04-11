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

pub(crate) use capabilities::platform_capabilities;
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
        crate::models::ClipboardBackend::Plugin => plugin_writer::write_payload(app, &payload),
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
