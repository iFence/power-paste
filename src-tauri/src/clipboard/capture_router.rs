use anyhow::Result;
use tauri::AppHandle;

use crate::{
    history::{build_captured_clipboard, should_ignore_app},
    models::{AppSettings, CapturedClipboard, ForegroundAppResult},
};

use super::plugin_reader::{read_snapshot, PluginClipboardImage};

fn image_path_from_files(files: &[String]) -> Option<String> {
    const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp"];

    files.iter().find_map(|path| {
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())?;
        IMAGE_EXTENSIONS
            .contains(&extension.as_str())
            .then(|| path.clone())
    })
}

fn image_from_file_path(path: &str) -> Option<PluginClipboardImage> {
    let image = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?
        .into_rgba8();
    let width = image.width();
    let height = image.height();
    let mut png_bytes = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .ok()?;

    Some(PluginClipboardImage {
        png_bytes,
        width,
        height,
    })
}

pub(crate) fn capture_clipboard(
    app: &AppHandle,
    settings: &AppSettings,
    source_app: Option<&ForegroundAppResult>,
) -> Result<Option<CapturedClipboard>> {
    if should_ignore_app(settings, source_app) {
        return Ok(None);
    }

    let snapshot = read_snapshot(app);
    let file_image = snapshot
        .image
        .is_none()
        .then(|| image_path_from_files(&snapshot.files))
        .flatten()
        .and_then(|path| image_from_file_path(&path));
    let image = snapshot.image.or(file_image);

    let image_bytes = image.as_ref().map(|image| image.png_bytes.clone());
    let width = image.as_ref().map(|image| image.width);
    let height = image.as_ref().map(|image| image.height);

    build_captured_clipboard(
        settings,
        snapshot.text.unwrap_or_default(),
        snapshot.html,
        snapshot.rtf,
        image_bytes,
        width,
        height,
    )
}
