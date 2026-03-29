use std::fs;

use anyhow::Result;
#[cfg(windows)]
use std::{thread, time::Duration};

use crate::models::{ClipboardTargetProfile, StoredClipboardItem};

#[cfg(windows)]
use crate::{
    clipboard_html::{build_mixed_item_html, ensure_cf_html},
    models::CF_DIB,
};
#[cfg(windows)]
use image::ImageReader;
#[cfg(windows)]
use std::{mem, os::windows::ffi::OsStrExt};
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::GlobalFree,
    Globalization::{WideCharToMultiByte, CP_ACP},
    Graphics::Gdi::{BITMAPINFOHEADER, BI_RGB},
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW,
            SetClipboardData,
        },
        Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
        Ole::{CF_TEXT, CF_UNICODETEXT},
    },
};

// Chooses the most appropriate clipboard payload per target app and item kind.
fn item_has_text(item: &StoredClipboardItem) -> bool {
    item.full_text
        .as_deref()
        .map(|text| !text.is_empty())
        .unwrap_or(false)
}

fn is_image_placeholder_text(text: &str) -> bool {
    let normalized = text.trim().to_lowercase();
    matches!(
        normalized.as_str(),
        "[é¥å‰§å¢–]" | "é¥å‰§å¢–" | "[image]" | "image" | "[img]" | "img"
    )
}

fn item_has_rich_text(item: &StoredClipboardItem) -> bool {
    item.html_text
        .as_deref()
        .map(|text| !text.is_empty())
        .unwrap_or(false)
        || item
            .rtf_text
            .as_deref()
            .map(|text| !text.is_empty())
            .unwrap_or(false)
}

fn item_has_textual_payload(item: &StoredClipboardItem) -> bool {
    item_has_text(item) || item_has_rich_text(item)
}

fn item_has_image(item: &StoredClipboardItem) -> bool {
    item.image_path
        .as_deref()
        .map(|path| !path.is_empty())
        .unwrap_or(false)
}

fn item_should_prefer_image_payload(item: &StoredClipboardItem) -> bool {
    item_has_image(item)
        && item
            .full_text
            .as_deref()
            .map(|text| {
                let normalized = text.trim();
                normalized == "[é¥å‰§å¢–]"
                    || normalized == "é¥å‰§å¢–"
                    || is_image_placeholder_text(normalized)
            })
            .unwrap_or(false)
}

#[cfg(windows)]
struct ClipboardGuard;

#[cfg(windows)]
impl ClipboardGuard {
    fn open() -> Result<Self> {
        // The clipboard is a shared global resource, so short retries avoid flaky failures.
        for _ in 0..10 {
            if unsafe { OpenClipboard(std::ptr::null_mut()) } != 0 {
                return Ok(Self);
            }
            thread::sleep(Duration::from_millis(5));
        }

        anyhow::bail!("failed to open clipboard")
    }
}

#[cfg(windows)]
impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

#[cfg(windows)]
// Plain text writes still go through the unified native payload writer so all clipboard setup stays in one place.
pub(crate) fn write_unicode_text_to_clipboard(text: &str) -> Result<()> {
    write_clipboard_payload_native(Some(text), None, None, None, None)
}

#[cfg(target_os = "macos")]
pub(crate) fn write_unicode_text_to_clipboard(text: &str) -> Result<()> {
    write_clipboard_payload_macos(Some(text), None, None)
}

#[cfg(target_os = "macos")]
fn temporary_clipboard_file(
    prefix: &str,
    extension: &str,
    bytes: &[u8],
) -> Result<std::path::PathBuf> {
    let path = std::env::temp_dir().join(format!(
        "power-paste-{prefix}-{}.{}",
        uuid::Uuid::new_v4(),
        extension
    ));
    fs::write(&path, bytes)?;
    Ok(path)
}

#[cfg(target_os = "macos")]
fn write_clipboard_payload_macos(
    text: Option<&str>,
    html: Option<&str>,
    image_path: Option<&str>,
) -> Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    const SCRIPT: &str = r#"
import AppKit
import Foundation

let args = Array(CommandLine.arguments.dropFirst())
let textPath = args.indices.contains(0) ? args[0] : ""
let htmlPath = args.indices.contains(1) ? args[1] : ""
let imagePath = args.indices.contains(2) ? args[2] : ""

let pasteboard = NSPasteboard.general
pasteboard.clearContents()

var wrote = false

if !textPath.isEmpty {
    let url = URL(fileURLWithPath: textPath)
    let text = try String(contentsOf: url, encoding: .utf8)
    pasteboard.setString(text, forType: .string)
    wrote = true
}

if !htmlPath.isEmpty {
    let url = URL(fileURLWithPath: htmlPath)
    let data = try Data(contentsOf: url)
    pasteboard.setData(data, forType: .html)
    wrote = true
}

if !imagePath.isEmpty {
    let url = URL(fileURLWithPath: imagePath)
    let data = try Data(contentsOf: url)
    pasteboard.setData(data, forType: NSPasteboard.PasteboardType("public.png"))
    wrote = true
}

if !wrote {
    fputs("clipboard payload is empty\n", stderr)
    exit(1)
}
"#;

    let text_path = text
        .filter(|value| !value.is_empty())
        .map(|value| temporary_clipboard_file("text", "txt", value.as_bytes()))
        .transpose()?;
    let html_path = html
        .filter(|value| !value.is_empty())
        .map(|value| temporary_clipboard_file("html", "html", value.as_bytes()))
        .transpose()?;
    let image_path = image_path
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let args = vec![
        "-".to_string(),
        text_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        html_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        image_path.unwrap_or_default(),
    ];

    let mut child = Command::new("swift")
        .env(
            "CLANG_MODULE_CACHE_PATH",
            std::env::temp_dir().join("power-paste-swift-cache"),
        )
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(SCRIPT.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if let Some(path) = text_path {
        let _ = fs::remove_file(path);
    }
    if let Some(path) = html_path {
        let _ = fs::remove_file(path);
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            anyhow::bail!("failed to write to macOS clipboard");
        }
        anyhow::bail!(stderr);
    }
    Ok(())
}

#[cfg(windows)]
fn register_clipboard_format(name: &str) -> Result<u32> {
    let wide: Vec<u16> = std::ffi::OsStr::new(name)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let format = unsafe { RegisterClipboardFormatW(wide.as_ptr()) };
    if format == 0 {
        anyhow::bail!("failed to register clipboard format: {name}");
    }
    Ok(format)
}

#[cfg(windows)]
fn set_clipboard_bytes(format: u32, bytes: &[u8]) -> Result<()> {
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) };
    if handle.is_null() {
        anyhow::bail!("failed to allocate clipboard buffer");
    }

    let result = (|| -> Result<()> {
        let target = unsafe { GlobalLock(handle) } as *mut u8;
        if target.is_null() {
            anyhow::bail!("failed to lock clipboard buffer");
        }

        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
            GlobalUnlock(handle);
        }

        if unsafe { SetClipboardData(format, handle) }.is_null() {
            anyhow::bail!("failed to set clipboard format {format}");
        }

        Ok(())
    })();

    if result.is_err() {
        unsafe {
            GlobalFree(handle);
        }
    }

    result
}

#[cfg(windows)]
fn set_clipboard_text_codepage(format: u32, text: &str) -> Result<()> {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);

    let required = unsafe {
        WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
            std::ptr::null_mut(),
        )
    };
    if required <= 0 {
        anyhow::bail!("failed to convert text to ANSI clipboard bytes");
    }

    let mut bytes = vec![0u8; required as usize];
    let written = unsafe {
        WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            bytes.as_mut_ptr(),
            bytes.len() as i32,
            std::ptr::null(),
            std::ptr::null_mut(),
        )
    };
    if written <= 0 {
        anyhow::bail!("failed to write ANSI clipboard bytes");
    }

    set_clipboard_bytes(format, &bytes)
}

#[cfg(windows)]
fn set_clipboard_utf8_text(format: u32, text: &str) -> Result<()> {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    set_clipboard_bytes(format, &bytes)
}

#[cfg(windows)]
fn set_clipboard_unicode_text(text: &str) -> Result<()> {
    let wide: Vec<u16> = std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let bytes = unsafe {
        std::slice::from_raw_parts(
            wide.as_ptr() as *const u8,
            wide.len() * mem::size_of::<u16>(),
        )
    };
    set_clipboard_bytes(CF_UNICODETEXT as u32, bytes)
}

#[cfg(windows)]
fn dib_bytes_from_image_path(image_path: &str) -> Result<Vec<u8>> {
    let rgba = ImageReader::open(image_path)?
        .with_guessed_format()?
        .decode()?
        .into_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    let pixel_bytes = rgba.into_raw();
    let mut dib_bytes = vec![0u8; mem::size_of::<BITMAPINFOHEADER>() + pixel_bytes.len()];

    let info = BITMAPINFOHEADER {
        biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width as i32,
        biHeight: height as i32,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        biSizeImage: pixel_bytes.len() as u32,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };

    unsafe {
        std::ptr::copy_nonoverlapping(
            &info as *const BITMAPINFOHEADER as *const u8,
            dib_bytes.as_mut_ptr(),
            mem::size_of::<BITMAPINFOHEADER>(),
        );
    }

    let target_pixels = &mut dib_bytes[mem::size_of::<BITMAPINFOHEADER>()..];
    let row_stride = width as usize * 4;
    for row in 0..height as usize {
        let src_row = height as usize - 1 - row;
        let src = &pixel_bytes[src_row * row_stride..(src_row + 1) * row_stride];
        let dest = &mut target_pixels[row * row_stride..(row + 1) * row_stride];
        for (column, pixel) in src.chunks_exact(4).enumerate() {
            let offset = column * 4;
            dest[offset] = pixel[2];
            dest[offset + 1] = pixel[1];
            dest[offset + 2] = pixel[0];
            dest[offset + 3] = pixel[3];
        }
    }

    Ok(dib_bytes)
}

#[cfg(windows)]
// Writes Unicode text, ANSI text, HTML, RTF and image formats in one clipboard transaction.
fn write_clipboard_payload_native(
    text: Option<&str>,
    html: Option<&str>,
    rtf: Option<&str>,
    png_bytes: Option<&[u8]>,
    image_path: Option<&str>,
) -> Result<()> {
    let _guard = ClipboardGuard::open()?;
    if unsafe { EmptyClipboard() } == 0 {
        anyhow::bail!("failed to clear clipboard");
    }

    if let Some(text) = text.filter(|value| !value.is_empty()) {
        set_clipboard_unicode_text(text)?;
        set_clipboard_text_codepage(CF_TEXT as u32, text)?;
    }

    if let Some(html) = html.filter(|value| !value.is_empty()) {
        let html_format = register_clipboard_format("HTML Format")?;
        set_clipboard_utf8_text(html_format, html)?;
    }

    if let Some(rtf) = rtf.filter(|value| !value.is_empty()) {
        let rtf_format = register_clipboard_format("Rich Text Format")?;
        set_clipboard_utf8_text(rtf_format, rtf)?;
    }

    if let Some(png_bytes) = png_bytes.filter(|value| !value.is_empty()) {
        let png_format = register_clipboard_format("PNG")?;
        set_clipboard_bytes(png_format, png_bytes)?;
    }

    if let Some(image_path) = image_path.filter(|value| !value.is_empty()) {
        let dib_bytes = dib_bytes_from_image_path(image_path)?;
        set_clipboard_bytes(CF_DIB, &dib_bytes)?;
    }

    Ok(())
}

#[cfg(windows)]
fn write_rich_text_to_clipboard_windows(item: &StoredClipboardItem) -> Result<()> {
    let html = item
        .html_text
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(ensure_cf_html);
    write_clipboard_payload_native(
        item.full_text.as_deref(),
        html.as_deref(),
        item.rtf_text.as_deref(),
        None,
        None,
    )
}

#[cfg(windows)]
// Mixed items keep both text and image payloads when the target app can consume both.
fn write_combined_item_to_clipboard_windows(
    item: &StoredClipboardItem,
    profile: ClipboardTargetProfile,
) -> Result<()> {
    let image_path = item
        .image_path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Image payload missing"))?;
    let png_bytes = fs::read(image_path)?;
    let html = build_mixed_item_html(item, profile);
    write_clipboard_payload_native(
        item.full_text.as_deref(),
        html.as_deref(),
        None,
        Some(&png_bytes),
        Some(image_path),
    )
}

#[cfg(windows)]
pub(crate) fn write_image_to_clipboard(image_path: &str) -> Result<()> {
    let png_bytes = fs::read(image_path)?;
    write_clipboard_payload_native(None, None, None, Some(&png_bytes), Some(image_path))
}

#[cfg(windows)]
fn write_image_payload_to_clipboard_windows(item: &StoredClipboardItem) -> Result<()> {
    let image_path = item
        .image_path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Image payload missing"))?;
    let png_bytes = fs::read(image_path)?;
    write_clipboard_payload_native(None, None, None, Some(&png_bytes), Some(image_path))
}

#[cfg(not(windows))]
fn write_image_payload_to_clipboard_windows(_item: &StoredClipboardItem) -> Result<()> {
    anyhow::bail!("unsupported_clipboard_write")
}

#[cfg(windows)]
fn write_item_to_clipboard_windows(
    item: &StoredClipboardItem,
    profile: ClipboardTargetProfile,
) -> Result<()> {
    if item.kind == "text" {
        if profile == ClipboardTargetProfile::Wps {
            return write_unicode_text_to_clipboard(item.full_text.as_deref().unwrap_or_default());
        }
        if item_has_rich_text(item) {
            return write_rich_text_to_clipboard_windows(item);
        }
        return write_unicode_text_to_clipboard(item.full_text.as_deref().unwrap_or_default());
    }

    if item_should_prefer_image_payload(item) {
        return write_image_payload_to_clipboard_windows(item);
    }

    if item.kind == "mixed" {
        if item_has_image(item) {
            return write_combined_item_to_clipboard_windows(item, profile);
        }

        if item_has_rich_text(item) {
            return write_rich_text_to_clipboard_windows(item);
        }

        return write_unicode_text_to_clipboard(item.full_text.as_deref().unwrap_or_default());
    }

    match (item_has_textual_payload(item), item_has_image(item)) {
        (true, true) => write_combined_item_to_clipboard_windows(item, profile),
        (true, false) => {
            write_unicode_text_to_clipboard(item.full_text.as_deref().unwrap_or_default())
        }
        (false, true) => write_image_payload_to_clipboard_windows(item),
        (false, false) => Ok(()),
    }
}

// Public clipboard write entrypoint used by commands and paste flows.
pub(crate) fn write_item_to_clipboard_with_profile(
    item: &StoredClipboardItem,
    profile: ClipboardTargetProfile,
) -> Result<()> {
    #[cfg(windows)]
    {
        return write_item_to_clipboard_windows(item, profile);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = profile;
        let html = item.html_text.as_deref().filter(|value| !value.is_empty());
        let image_path = item.image_path.as_deref().filter(|value| !value.is_empty());

        if item_should_prefer_image_payload(item) {
            return write_clipboard_payload_macos(None, None, image_path);
        }

        if item.kind == "mixed" {
            return write_clipboard_payload_macos(item.full_text.as_deref(), html, image_path);
        }

        match (item_has_textual_payload(item), item_has_image(item)) {
            (true, true) => {
                write_clipboard_payload_macos(item.full_text.as_deref(), html, image_path)
            }
            (true, false) => write_clipboard_payload_macos(item.full_text.as_deref(), html, None),
            (false, true) => write_clipboard_payload_macos(None, None, image_path),
            (false, false) => Ok(()),
        }
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = item;
        let _ = profile;
        anyhow::bail!("unsupported_clipboard_write")
    }
}
