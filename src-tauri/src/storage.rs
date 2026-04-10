use std::fs;

use anyhow::Result;
use image::load_from_memory;
use serde_json::{from_slice, to_vec_pretty};
use sha2::{Digest, Sha256};

use crate::models::{AppSettings, StoragePaths};

pub(crate) fn load_settings(paths: &StoragePaths) -> Result<AppSettings> {
    if !paths.settings_path.exists() {
        let settings = AppSettings::default();
        save_settings(paths, &settings)?;
        return Ok(settings);
    }

    let bytes = fs::read(&paths.settings_path)?;
    let mut settings: AppSettings = from_slice(&bytes)?;
    settings.polling_interval_ms = 500;
    Ok(settings)
}

pub(crate) fn save_settings(paths: &StoragePaths, settings: &AppSettings) -> Result<()> {
    fs::write(&paths.settings_path, to_vec_pretty(settings)?)?;
    Ok(())
}

pub(crate) fn preview_text(text: &str) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        "(empty text)".into()
    } else {
        normalized.chars().take(160).collect()
    }
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub(crate) fn text_hash(text: &str, html_text: Option<&str>, rtf_text: Option<&str>) -> String {
    if !text.is_empty() {
        return sha256_hex(text.as_bytes());
    }
    if let Some(html) = html_text.filter(|value| !value.is_empty()) {
        return sha256_hex(html.as_bytes());
    }
    if let Some(rtf) = rtf_text.filter(|value| !value.is_empty()) {
        return sha256_hex(rtf.as_bytes());
    }
    sha256_hex(b"")
}

pub(crate) fn image_hash_from_png_bytes(png_bytes: &[u8]) -> Result<String> {
    let image = load_from_memory(png_bytes)?;
    let rgba = image.into_rgba8();
    let (width, height) = rgba.dimensions();
    let mut hasher = Sha256::new();
    hasher.update(width.to_le_bytes());
    hasher.update(height.to_le_bytes());
    hasher.update(rgba.as_raw());
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn mixed_hash(
    text: &str,
    html_text: Option<&str>,
    rtf_text: Option<&str>,
    png_bytes: &[u8],
) -> Result<String> {
    let image_hash = image_hash_from_png_bytes(png_bytes)?;
    let text_fingerprint = if !text.is_empty() {
        text.to_string()
    } else if let Some(html) = html_text.filter(|value| !value.is_empty()) {
        html.to_string()
    } else if let Some(rtf) = rtf_text.filter(|value| !value.is_empty()) {
        rtf.to_string()
    } else {
        String::new()
    };
    Ok(sha256_hex(
        format!("{text_fingerprint}\n{image_hash}").as_bytes(),
    ))
}
