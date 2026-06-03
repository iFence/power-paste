use std::fs;

use anyhow::Result;
use image::{load_from_memory, ImageFormat};
use serde_json::{from_slice, to_vec_pretty};
use sha2::{Digest, Sha256};

use crate::models::{AppSettings, StoragePaths};

pub(crate) fn load_settings(paths: &StoragePaths) -> Result<AppSettings> {
    if !paths.settings_path.exists() {
        let settings = AppSettings::default().normalized();
        save_settings(paths, &settings)?;
        return Ok(settings);
    }

    let bytes = fs::read(&paths.settings_path)?;
    let settings: AppSettings = from_slice(&bytes)?;
    let settings = settings.normalized();
    Ok(settings)
}

pub(crate) fn save_settings(paths: &StoragePaths, settings: &AppSettings) -> Result<()> {
    let settings = settings.clone().normalized();
    if let Some(parent) = paths.settings_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&paths.settings_path, to_vec_pretty(&settings)?)?;
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
    let mut hasher = Sha256::new();
    hasher.update((png_bytes.len() as u64).to_le_bytes());
    hasher.update(png_bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn image_preview_png_from_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
    const PREVIEW_MAX_WIDTH: u32 = 420;
    const PREVIEW_MAX_HEIGHT: u32 = 320;
    const INLINE_ORIGINAL_LIMIT: usize = 256 * 1024;

    if bytes.len() <= INLINE_ORIGINAL_LIMIT
        && matches!(image::guess_format(bytes).ok(), Some(ImageFormat::Png))
    {
        return Some(bytes.to_vec());
    }

    let image = load_from_memory(bytes).ok()?;
    let preview = image.thumbnail(PREVIEW_MAX_WIDTH, PREVIEW_MAX_HEIGHT);
    let mut preview_bytes = Vec::new();
    preview
        .write_to(
            &mut std::io::Cursor::new(&mut preview_bytes),
            ImageFormat::Png,
        )
        .ok()?;

    Some(preview_bytes)
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

#[cfg(test)]
mod tests {
    use super::{load_settings, save_settings};
    use crate::models::{AppSettings, StoragePaths};
    use std::fs;
    use uuid::Uuid;

    fn test_paths() -> StoragePaths {
        let root = std::env::temp_dir().join(format!("clipdesk-storage-test-{}", Uuid::new_v4()));
        StoragePaths::new(root).expect("storage paths")
    }

    #[test]
    fn preserves_polling_interval_on_roundtrip() {
        let paths = test_paths();
        let mut settings = AppSettings::default();
        settings.polling_interval_ms = 1250;
        settings
            .tag_labels
            .insert("orange".into(), "紧急".into());

        save_settings(&paths, &settings).expect("save settings");
        let loaded = load_settings(&paths).expect("load settings");

        assert_eq!(loaded.polling_interval_ms, 1250);
        assert_eq!(loaded.tag_labels.get("orange").map(String::as_str), Some("紧急"));

        let _ = fs::remove_dir_all(
            paths
                .settings_path
                .parent()
                .unwrap_or(paths.settings_path.as_path()),
        );
    }

    #[test]
    fn returns_error_for_invalid_settings_file() {
        let paths = test_paths();

        fs::write(&paths.settings_path, b"{invalid json").expect("write invalid settings");
        let result = load_settings(&paths);

        assert!(result.is_err());

        let _ = fs::remove_dir_all(
            paths
                .settings_path
                .parent()
                .unwrap_or(paths.settings_path.as_path()),
        );
    }

    #[test]
    fn recreates_parent_directory_before_saving_settings() {
        let paths = test_paths();
        let parent = paths
            .settings_path
            .parent()
            .unwrap_or(paths.settings_path.as_path())
            .to_path_buf();
        let mut settings = AppSettings::default();
        settings.polling_interval_ms = 1750;

        fs::remove_dir_all(&parent).expect("remove settings parent");
        save_settings(&paths, &settings).expect("save settings after parent recreation");

        let loaded = load_settings(&paths).expect("load settings after parent recreation");
        assert_eq!(loaded.polling_interval_ms, 1750);

        let _ = fs::remove_dir_all(parent);
    }
}
