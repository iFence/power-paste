use std::fs;

use anyhow::Result;

use crate::{models::StoredClipboardItem, clipboard::payload::ClipboardPayload};

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

fn write_clipboard_payload_macos(
    text: Option<&str>,
    html: Option<&str>,
    image_png: Option<&[u8]>,
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
    let image_path = image_png
        .filter(|value| !value.is_empty())
        .map(|value| temporary_clipboard_file("image", "png", value))
        .transpose()?;

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
        image_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
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
    if let Some(path) = image_path {
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

pub(crate) fn write_payload(payload: &ClipboardPayload) -> Result<()> {
    match payload {
        ClipboardPayload::Mixed {
            text,
            html,
            png_bytes,
        } => write_clipboard_payload_macos(text.as_deref(), html.as_deref(), Some(png_bytes)),
        _ => anyhow::bail!("clipboard payload requires plugin writer"),
    }
}

pub(crate) fn write_image_to_clipboard(_png_bytes: &[u8]) -> Result<()> {
    anyhow::bail!("unsupported_clipboard_write")
}

#[allow(dead_code)]
pub(crate) fn _assert_item_type(_item: &StoredClipboardItem) {}
