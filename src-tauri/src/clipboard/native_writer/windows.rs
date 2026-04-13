use anyhow::Result;
use image::load_from_memory;
use std::{mem, os::windows::ffi::OsStrExt, thread, time::Duration};
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

use crate::{
    clipboard_html::{build_mixed_item_html, ensure_cf_html},
    models::{StoredClipboardItem, CF_DIB},
    paste_target::TargetProfile,
};

use super::super::payload::ClipboardPayload;

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

fn set_clipboard_utf8_text(format: u32, text: &str) -> Result<()> {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    set_clipboard_bytes(format, &bytes)
}

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

fn dib_bytes_from_png_bytes(png_bytes: &[u8]) -> Result<Vec<u8>> {
    let rgba = load_from_memory(png_bytes)?.into_rgba8();
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

fn write_clipboard_payload_native(
    text: Option<&str>,
    html: Option<&str>,
    png_bytes: Option<&[u8]>,
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

    if let Some(png_bytes) = png_bytes.filter(|value| !value.is_empty()) {
        let png_format = register_clipboard_format("PNG")?;
        set_clipboard_bytes(png_format, png_bytes)?;
        let dib_bytes = dib_bytes_from_png_bytes(png_bytes)?;
        set_clipboard_bytes(CF_DIB, &dib_bytes)?;
    }

    Ok(())
}

pub(crate) fn write_mixed_payload(
    item: &StoredClipboardItem,
    profile: TargetProfile,
) -> Result<()> {
    let html = build_mixed_item_html(item, profile);
    write_clipboard_payload_native(item.full_text.as_deref(), html.as_deref(), item.image_png.as_deref())
}

pub(crate) fn write_payload(
    item: &StoredClipboardItem,
    profile: TargetProfile,
    payload: &ClipboardPayload,
) -> Result<()> {
    match payload {
        ClipboardPayload::Empty => Ok(()),
        ClipboardPayload::Text { text } => write_clipboard_payload_native(Some(text), None, None),
        ClipboardPayload::Html { text, html } => {
            let html = ensure_cf_html(html);
            write_clipboard_payload_native(text.as_deref(), Some(html.as_str()), None)
        }
        ClipboardPayload::Image { png_bytes } => {
            write_clipboard_payload_native(None, None, Some(png_bytes))
        }
        ClipboardPayload::RichText { text, html, rtf: _ } => {
            if let Some(html) = html {
                let html = ensure_cf_html(html);
                write_clipboard_payload_native(text.as_deref(), Some(html.as_str()), None)
            } else if let Some(text) = text {
                write_clipboard_payload_native(Some(text), None, None)
            } else {
                Ok(())
            }
        }
        ClipboardPayload::Mixed { .. } => write_mixed_payload(item, profile),
    }
}

pub(crate) fn write_image_to_clipboard(png_bytes: &[u8]) -> Result<()> {
    write_clipboard_payload_native(None, None, Some(png_bytes))
}
