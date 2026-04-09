use std::fs;

use anyhow::Result;
use tauri::AppHandle;
use tauri_plugin_clipboard_next::ClipboardNextExt;
use uuid::Uuid;

use super::payload::ClipboardPayload;

fn write_png_bytes_to_temp_file(png_bytes: &[u8]) -> Result<String> {
    let dir = std::env::temp_dir().join("clipdesk-clipboard");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.png", Uuid::new_v4()));
    fs::write(&path, png_bytes)?;
    Ok(path.to_string_lossy().to_string())
}

#[cfg(windows)]
pub(crate) fn write_text(app: &AppHandle, text: &str) -> Result<()> {
    app.clipboard_next()
        .write_text(text.to_string())
        .map_err(anyhow::Error::msg)
}

pub(crate) fn write_image(app: &AppHandle, png_bytes: &[u8]) -> Result<()> {
    let image_path = write_png_bytes_to_temp_file(png_bytes)?;
    app.clipboard_next()
        .write_image(image_path)
        .map_err(anyhow::Error::msg)
}

pub(crate) fn preferred_payload(payload: &ClipboardPayload) -> ClipboardPayload {
    match payload {
        ClipboardPayload::Mixed {
            text,
            html,
            png_bytes,
        } if cfg!(target_os = "macos") => {
            if let Some(html) = html.clone().filter(|value| !value.trim().is_empty()) {
                ClipboardPayload::Html {
                    text: text.clone().filter(|value| !value.trim().is_empty()),
                    html,
                }
            } else if let Some(text) = text.clone().filter(|value| !value.trim().is_empty()) {
                ClipboardPayload::Text { text }
            } else {
                ClipboardPayload::Image {
                    png_bytes: png_bytes.clone(),
                }
            }
        }
        other => other.clone(),
    }
}

pub(crate) fn write_payload(app: &AppHandle, payload: &ClipboardPayload) -> Result<ClipboardPayload> {
    let clipboard = app.clipboard_next();
    let payload = preferred_payload(payload);

    match &payload {
        ClipboardPayload::Empty => Ok(payload),
        ClipboardPayload::Text { text } => clipboard
            .write_text(text.clone())
            .map_err(anyhow::Error::msg)
            .map(|_| payload),
        ClipboardPayload::Html { html, .. } => clipboard
            .write_html(html.clone())
            .map_err(anyhow::Error::msg)
            .map(|_| payload),
        ClipboardPayload::Image { png_bytes } => write_image(app, png_bytes).map(|_| payload),
        ClipboardPayload::RichText { rtf, html, text } => {
            if let Some(rtf) = rtf {
                clipboard
                    .write_rtf(rtf.clone())
                    .map_err(anyhow::Error::msg)
                    .map(|_| payload)
            } else if let Some(html) = html {
                clipboard
                    .write_html(html.clone())
                    .map_err(anyhow::Error::msg)
                    .map(|_| payload)
            } else if let Some(text) = text {
                clipboard
                    .write_text(text.clone())
                    .map_err(anyhow::Error::msg)
                    .map(|_| payload)
            } else {
                Ok(ClipboardPayload::Empty)
            }
        }
        ClipboardPayload::Mixed { .. } => anyhow::bail!("clipboard payload requires native writer"),
    }
}

#[cfg(test)]
mod tests {
    use super::preferred_payload;
    use crate::clipboard::payload::ClipboardPayload;

    #[test]
    fn prefers_html_for_mixed_payload_on_macos() {
        let payload = ClipboardPayload::Mixed {
            text: Some("plain".into()),
            html: Some("<b>plain</b>".into()),
            png_bytes: vec![1, 2, 3],
        };

        let preferred = preferred_payload(&payload);

        if cfg!(target_os = "macos") {
            assert!(matches!(preferred, ClipboardPayload::Html { .. }));
        } else {
            assert!(matches!(preferred, ClipboardPayload::Mixed { .. }));
        }
    }
}
