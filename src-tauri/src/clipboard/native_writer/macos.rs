use anyhow::Result;
use core::ffi::c_void;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{
    NSPasteboard, NSPasteboardItem, NSPasteboardTypeHTML, NSPasteboardTypePNG, NSPasteboardTypeRTF,
    NSPasteboardTypeString,
};
use objc2_foundation::{NSArray, NSData, NSString};

use crate::{clipboard::payload::ClipboardPayload, models::StoredClipboardItem};

fn write_clipboard_payload_macos(
    text: Option<&str>,
    rtf: Option<&str>,
    html: Option<&str>,
    image_png: Option<&[u8]>,
) -> Result<()> {
    // 直接在 Rust 进程内写入 NSPasteboard，避免外部启动 swift 和临时文件带来的额外延迟。
    unsafe {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();

        let item = NSPasteboardItem::new();
        let mut has_content = false;

        if let Some(text) = text.filter(|value| !value.is_empty()) {
            if !item.setString_forType(&NSString::from_str(text), NSPasteboardTypeString) {
                anyhow::bail!("failed to write text to macOS clipboard");
            }
            has_content = true;
        }

        if let Some(rtf) = rtf.filter(|value| !value.is_empty()) {
            let data = NSData::dataWithBytes_length(rtf.as_ptr() as *const c_void, rtf.len());
            if !item.setData_forType(&data, NSPasteboardTypeRTF) {
                anyhow::bail!("failed to write rtf to macOS clipboard");
            }
            has_content = true;
        }

        if let Some(html) = html.filter(|value| !value.is_empty()) {
            if !item.setString_forType(&NSString::from_str(html), NSPasteboardTypeHTML) {
                anyhow::bail!("failed to write html to macOS clipboard");
            }
            has_content = true;
        }

        if let Some(image_png) = image_png.filter(|value| !value.is_empty()) {
            let data =
                NSData::dataWithBytes_length(image_png.as_ptr() as *const c_void, image_png.len());
            if !item.setData_forType(&data, NSPasteboardTypePNG) {
                anyhow::bail!("failed to write image to macOS clipboard");
            }
            has_content = true;
        }

        if !has_content {
            anyhow::bail!("clipboard payload is empty");
        }

        let objects = NSArray::from_retained_slice(&[ProtocolObject::from_retained(item)]);
        if !pasteboard.writeObjects(&objects) {
            anyhow::bail!("failed to commit macOS clipboard payload");
        }
    }

    Ok(())
}

pub(crate) fn write_payload(payload: &ClipboardPayload) -> Result<()> {
    match payload {
        ClipboardPayload::Html { text, html } => {
            write_clipboard_payload_macos(text.as_deref(), None, Some(html.as_str()), None)
        }
        ClipboardPayload::Image { png_bytes } => {
            write_clipboard_payload_macos(None, None, None, Some(png_bytes))
        }
        ClipboardPayload::Mixed {
            text,
            html,
            png_bytes,
        } => write_clipboard_payload_macos(text.as_deref(), None, html.as_deref(), Some(png_bytes)),
        ClipboardPayload::RichText { text, rtf, html } => {
            write_clipboard_payload_macos(text.as_deref(), rtf.as_deref(), html.as_deref(), None)
        }
        _ => anyhow::bail!("clipboard payload requires plugin writer"),
    }
}

#[allow(dead_code)]
pub(crate) fn _assert_item_type(_item: &StoredClipboardItem) {}
