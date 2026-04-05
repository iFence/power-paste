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

pub(crate) fn write_payload(app: &AppHandle, payload: &ClipboardPayload) -> Result<()> {
    let clipboard = app.clipboard_next();

    match payload {
        ClipboardPayload::Empty => Ok(()),
        ClipboardPayload::Text { text } => {
            clipboard.write_text(text.clone()).map_err(anyhow::Error::msg)
        }
        ClipboardPayload::Html { html, .. } => {
            clipboard.write_html(html.clone()).map_err(anyhow::Error::msg)
        }
        ClipboardPayload::Image { png_bytes } => write_image(app, png_bytes),
        ClipboardPayload::RichText { rtf, html, text } => {
            if let Some(rtf) = rtf {
                clipboard.write_rtf(rtf.clone()).map_err(anyhow::Error::msg)
            } else if let Some(html) = html {
                clipboard.write_html(html.clone()).map_err(anyhow::Error::msg)
            } else if let Some(text) = text {
                clipboard.write_text(text.clone()).map_err(anyhow::Error::msg)
            } else {
                Ok(())
            }
        }
        ClipboardPayload::Mixed { .. } => anyhow::bail!("clipboard payload requires native writer"),
    }
}
