use std::path::Path;

use anyhow::Result;
use tauri::AppHandle;
use tauri_plugin_clipboard_next::ClipboardNextExt;

#[derive(Debug)]
pub(crate) struct PluginClipboardImage {
    pub(crate) png_bytes: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug, Default)]
pub(crate) struct PluginClipboardSnapshot {
    pub(crate) text: Option<String>,
    pub(crate) html: Option<String>,
    pub(crate) rtf: Option<String>,
    pub(crate) image: Option<PluginClipboardImage>,
    pub(crate) files: Vec<String>,
}

pub(crate) fn normalize_text(text: String) -> Option<String> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    (!normalized.trim().is_empty()).then_some(normalized)
}

fn image_from_plugin_read(path: &Path, width: u32, height: u32) -> Result<PluginClipboardImage> {
    Ok(PluginClipboardImage {
        png_bytes: std::fs::read(path)?,
        width,
        height,
    })
}

pub(crate) fn read_snapshot(app: &AppHandle) -> PluginClipboardSnapshot {
    let clipboard = app.clipboard_next();
    let text = clipboard.read_text().ok().and_then(normalize_text);
    let html = clipboard.read_html().ok().and_then(normalize_text);
    let rtf = clipboard.read_rtf().ok().and_then(normalize_text);
    let image = clipboard
        .read_image(app.clone(), None)
        .ok()
        .and_then(|image| image_from_plugin_read(&image.path, image.width, image.height).ok());
    let files = clipboard
        .read_files()
        .ok()
        .map(|files| files.files.into_iter().map(|file| file.path).collect())
        .unwrap_or_default();

    PluginClipboardSnapshot {
        text,
        html,
        rtf,
        image,
        files,
    }
}
