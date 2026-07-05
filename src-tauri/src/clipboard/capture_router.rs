use anyhow::Result;
use std::io::Cursor;
use tauri::AppHandle;

use crate::{
    history::{build_captured_clipboard, should_ignore_app},
    models::{AppSettings, CapturedClipboard, ForegroundAppResult},
};

use super::plugin_reader::{read_snapshot, PluginClipboardImage, PluginClipboardSnapshot};

#[cfg(windows)]
mod windows_native_reader {
    use super::*;
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt, thread, time::Duration};
    use windows_sys::Win32::System::{
        DataExchange::{
            CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
            RegisterClipboardFormatW,
        },
        Memory::{GlobalLock, GlobalSize, GlobalUnlock},
        Ole::CF_UNICODETEXT,
    };

    struct ClipboardGuard;

    impl ClipboardGuard {
        fn open() -> Result<Self> {
            for _ in 0..10 {
                if unsafe { OpenClipboard(std::ptr::null_mut()) } != 0 {
                    return Ok(Self);
                }
                thread::sleep(Duration::from_millis(5));
            }

            anyhow::bail!("failed to open clipboard")
        }
    }

    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            unsafe {
                CloseClipboard();
            }
        }
    }

    fn register_clipboard_format(name: &str) -> Option<u32> {
        let wide: Vec<u16> = OsStr::new(name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let format = unsafe { RegisterClipboardFormatW(wide.as_ptr()) };
        (format != 0).then_some(format)
    }

    fn read_format_bytes(format: u32) -> Option<Vec<u8>> {
        if unsafe { IsClipboardFormatAvailable(format) } == 0 {
            return None;
        }

        let handle = unsafe { GetClipboardData(format) };
        if handle.is_null() {
            return None;
        }

        let size = unsafe { GlobalSize(handle) };
        if size == 0 {
            return None;
        }

        let source = unsafe { GlobalLock(handle) } as *const u8;
        if source.is_null() {
            return None;
        }

        let bytes = unsafe { std::slice::from_raw_parts(source, size) }.to_vec();
        unsafe {
            GlobalUnlock(handle);
        }

        Some(bytes).filter(|bytes| !bytes.is_empty())
    }

    fn dimensions_from_encoded(bytes: &[u8]) -> Option<(u32, u32)> {
        let format = image::guess_format(bytes).ok()?;
        image::ImageReader::with_format(Cursor::new(bytes), format)
            .into_dimensions()
            .ok()
    }

    fn png_from_encoded(bytes: &[u8], mime: &str) -> Option<Vec<u8>> {
        if mime == "image/png" {
            return Some(bytes.to_vec());
        }

        let image = image::load_from_memory(bytes).ok()?;
        let mut png_bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
            .ok()?;
        Some(png_bytes)
    }

    fn read_unicode_text() -> Option<String> {
        let bytes = read_format_bytes(CF_UNICODETEXT as u32)?;
        let words = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .take_while(|value| *value != 0)
            .collect::<Vec<_>>();
        String::from_utf16(&words)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    fn read_html() -> Option<String> {
        let format = register_clipboard_format("HTML Format")?;
        let bytes = read_format_bytes(format)?;
        let text = String::from_utf8_lossy(&bytes)
            .trim_matches('\0')
            .trim()
            .to_string();
        (!text.is_empty()).then_some(text)
    }

    fn read_encoded_image() -> Option<PluginClipboardImage> {
        for (format_name, mime) in [
            ("PNG", "image/png"),
            ("JFIF", "image/jpeg"),
            ("GIF", "image/gif"),
        ] {
            let Some(format) = register_clipboard_format(format_name) else {
                continue;
            };
            let Some(bytes) = read_format_bytes(format) else {
                continue;
            };
            let Some((width, height)) = dimensions_from_encoded(&bytes) else {
                continue;
            };
            let Some(png_bytes) = png_from_encoded(&bytes, mime) else {
                continue;
            };

            return Some(PluginClipboardImage {
                png_bytes,
                original_bytes: Some(bytes),
                original_mime: Some(mime.to_string()),
                width,
                height,
            });
        }

        None
    }

    pub(super) fn read_snapshot() -> Option<PluginClipboardSnapshot> {
        let _guard = ClipboardGuard::open().ok()?;
        Some(PluginClipboardSnapshot {
            text: read_unicode_text(),
            html: read_html(),
            rtf: None,
            image: read_encoded_image(),
            files: Vec::new(),
        })
    }
}

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
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .ok()?;

    Some(PluginClipboardImage {
        png_bytes,
        original_bytes: None,
        original_mime: None,
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

    #[cfg(windows)]
    let snapshot = {
        let mut snapshot = windows_native_reader::read_snapshot().unwrap_or_default();
        if snapshot.text.is_none() && snapshot.html.is_none() && snapshot.image.is_none() {
            snapshot = read_snapshot(app, true);
        }
        snapshot
    };
    #[cfg(not(windows))]
    let mut snapshot = read_snapshot(app, true);
    let file_image = snapshot
        .image
        .is_none()
        .then(|| image_path_from_files(&snapshot.files))
        .flatten()
        .and_then(|path| image_from_file_path(&path));
    let image = snapshot.image.or(file_image);

    let (image_bytes, original_bytes, original_mime, width, height) = match image {
        Some(image) => (
            Some(image.png_bytes),
            image.original_bytes,
            image.original_mime,
            Some(image.width),
            Some(image.height),
        ),
        None => (None, None, None, None, None),
    };

    build_captured_clipboard(
        settings,
        snapshot.text.unwrap_or_default(),
        snapshot.html,
        snapshot.rtf,
        image_bytes,
        original_bytes,
        original_mime,
        width,
        height,
    )
}
