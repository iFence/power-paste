use std::{collections::HashMap, fs, thread, time::Duration};

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
#[cfg(target_os = "macos")]
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

use crate::{
    models::{
        AppSettings, CapturedClipboard, ClipboardItemDto, ForegroundAppResult,
        PowershellClipboardResult, StoragePaths, StoredClipboardItem,
    },
    storage::{image_hash_from_png_bytes, mixed_hash, preview_text, save_history, text_hash},
};

#[cfg(target_os = "macos")]
fn run_macos_command(program: &str, args: &[&str]) -> Result<Option<String>> {
    let output = std::process::Command::new(program).args(args).output()?;
    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8(output.stdout)?;
    let trimmed = stdout.trim_end_matches(['\r', '\n']);
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

#[cfg(target_os = "macos")]
static MACOS_APP_ICON_CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();

#[cfg(target_os = "macos")]
fn parse_lsappinfo_field(output: &str, key: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let prefix = format!("\"{key}\"=");
        line.strip_prefix(&prefix)
            .map(str::trim)
            .and_then(|value| value.strip_prefix('"'))
            .and_then(|value| value.strip_suffix('"'))
            .map(ToString::to_string)
    })
}

#[cfg(target_os = "macos")]
fn macos_app_icon_base64(app_path: &str) -> Option<String> {
    let cache = MACOS_APP_ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(icon) = cache.lock().unwrap().get(app_path).cloned() {
        return icon;
    }

    const SCRIPT: &str = r#"
function run(argv) {
  ObjC.import('AppKit');
  const path = argv[0];
  const ws = $.NSWorkspace.sharedWorkspace;
  const image = ws.iconForFile(path);
  if (!image) {
    return '';
  }
  const tiff = image.TIFFRepresentation;
  if (!tiff) {
    return '';
  }
  const rep = $.NSBitmapImageRep.imageRepWithData(tiff);
  if (!rep) {
    return '';
  }
  const png = rep.representationUsingTypeProperties($.NSBitmapImageFileTypePNG, $({}));
  if (!png) {
    return '';
  }
  return ObjC.unwrap(png.base64EncodedStringWithOptions(0));
}
"#;

    let icon = std::process::Command::new("osascript")
        .args(["-l", "JavaScript", "-e", SCRIPT, app_path])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|output| output.trim().to_string())
        .filter(|output| !output.is_empty());

    cache
        .lock()
        .unwrap()
        .insert(app_path.to_string(), icon.clone());
    icon
}

// Normalizes raw process names into labels that make sense in the UI.
fn friendly_process_name(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "excel" => "Excel".into(),
        "winword" => "Word".into(),
        "powerpnt" => "PowerPoint".into(),
        "onenote" => "OneNote".into(),
        "typora" => "Typora".into(),
        "code" => "VS Code".into(),
        "notepad" => "Notepad".into(),
        "notepad++" => "Notepad++".into(),
        "chrome" => "Google Chrome".into(),
        "msedge" => "Microsoft Edge".into(),
        "firefox" => "Firefox".into(),
        "wechat" => "WeChat".into(),
        "qq" => "QQ".into(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

fn source_app_label(app: ForegroundAppResult) -> Option<String> {
    let display_name = app.display_name.trim();
    if !display_name.is_empty() && !display_name.eq_ignore_ascii_case("Program Manager") {
        return Some(display_name.to_string());
    }

    let process_name = app.process_name.trim();
    if process_name.is_empty() {
        None
    } else {
        Some(friendly_process_name(process_name))
    }
}

pub(crate) fn source_app_info(app: ForegroundAppResult) -> Option<(String, Option<String>)> {
    let label = source_app_label(ForegroundAppResult {
        process_name: app.process_name.clone(),
        display_name: app.display_name.clone(),
        icon_png_base64: app.icon_png_base64.clone(),
        app_path: app.app_path.clone(),
    })?;
    let icon_base64 = app
        .icon_png_base64
        .filter(|value| !value.is_empty())
        .or_else(|| {
            #[cfg(target_os = "macos")]
            {
                app.app_path.as_deref().and_then(macos_app_icon_base64)
            }
            #[cfg(not(target_os = "macos"))]
            {
                None
            }
        });
    let icon = icon_base64.map(|value| format!("data:image/png;base64,{value}"));
    Some((label, icon))
}

// Clipboard APIs on Windows can briefly lock; retrying avoids dropping valid captures.
fn is_clipboard_busy_error(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("clipboard")
        && (lower.contains("failed")
            || lower.contains("operation")
            || lower.contains("currently unavailable")
            || lower.contains("cannot open"))
        || lower.contains("busy")
        || lower.contains("unavailable")
        || lower.contains("denied")
}

fn powershell_clipboard_retry(script: &str) -> Result<Option<String>> {
    let mut last_error = None;
    for _ in 0..6 {
        match crate::powershell(script) {
            Ok(output) => return Ok(Some(output)),
            Err(error) => {
                let message = error.to_string();
                if !is_clipboard_busy_error(&message) {
                    return Err(error);
                }
                last_error = Some(message);
                thread::sleep(Duration::from_millis(15));
            }
        }
    }

    if let Some(message) = last_error {
        if is_clipboard_busy_error(&message) {
            return Ok(None);
        }
    }

    Ok(None)
}

// PowerShell is used here because Windows clipboard APIs expose HTML/RTF/image combinations most reliably that way.
fn capture_clipboard_payload() -> Result<Option<PowershellClipboardResult>> {
    let Some(output) = powershell_clipboard_retry(
        "$ErrorActionPreference='Stop'; \
         [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
         Add-Type -AssemblyName System.Windows.Forms; \
         Add-Type -AssemblyName System.Drawing; \
         $data = [System.Windows.Forms.Clipboard]::GetDataObject(); \
         if ($null -eq $data) { return }; \
         $text = ''; \
         $html = ''; \
         $rtf = ''; \
         $pngBase64 = $null; \
         $width = $null; \
         $height = $null; \
         if ($data.GetDataPresent([System.Windows.Forms.DataFormats]::UnicodeText)) { \
           $text = [string]$data.GetData([System.Windows.Forms.DataFormats]::UnicodeText) \
         } elseif ([System.Windows.Forms.Clipboard]::ContainsText()) { \
           $text = [System.Windows.Forms.Clipboard]::GetText() \
         } \
         if ($data.GetDataPresent([System.Windows.Forms.DataFormats]::Html)) { \
           $html = [string]$data.GetData([System.Windows.Forms.DataFormats]::Html) \
         } elseif ([System.Windows.Forms.Clipboard]::ContainsText([System.Windows.Forms.TextDataFormat]::Html)) { \
           $html = [System.Windows.Forms.Clipboard]::GetText([System.Windows.Forms.TextDataFormat]::Html) \
         } \
         if ($data.GetDataPresent([System.Windows.Forms.DataFormats]::Rtf)) { \
           $rtf = [string]$data.GetData([System.Windows.Forms.DataFormats]::Rtf) \
         } elseif ([System.Windows.Forms.Clipboard]::ContainsText([System.Windows.Forms.TextDataFormat]::Rtf)) { \
           $rtf = [System.Windows.Forms.Clipboard]::GetText([System.Windows.Forms.TextDataFormat]::Rtf) \
         } \
         $pngBytesValue = $null; \
         $img = $null; \
         if ($data.GetDataPresent([System.Windows.Forms.DataFormats]::Bitmap)) { \
           $img = $data.GetData([System.Windows.Forms.DataFormats]::Bitmap) \
         } elseif ([System.Windows.Forms.Clipboard]::ContainsImage()) { \
           $img = [System.Windows.Forms.Clipboard]::GetImage() \
         } \
         if ($null -eq $img) { \
           foreach ($format in @('PNG', 'image/png')) { \
             if (-not $data.GetDataPresent($format)) { continue } \
             $raw = $data.GetData($format, $false); \
             if ($raw -is [byte[]]) { \
               $pngBytesValue = $raw; \
               break \
             } \
             if ($raw -is [System.IO.MemoryStream]) { \
               $pngBytesValue = $raw.ToArray(); \
               break \
             } \
             if ($raw -is [System.IO.Stream]) { \
               $copyStream = New-Object System.IO.MemoryStream; \
               if ($raw.CanSeek) { [void]$raw.Seek(0, [System.IO.SeekOrigin]::Begin) } \
               $raw.CopyTo($copyStream); \
               $pngBytesValue = $copyStream.ToArray(); \
               $copyStream.Dispose(); \
               break \
             } \
           } \
         } \
         if ($null -ne $img) { \
           $ms = New-Object System.IO.MemoryStream; \
           $img.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png); \
           $pngBase64 = [Convert]::ToBase64String($ms.ToArray()); \
           $width = $img.Width; \
           $height = $img.Height; \
           $ms.Dispose(); \
           $img.Dispose(); \
         } elseif ($null -ne $pngBytesValue) { \
           $ms = New-Object System.IO.MemoryStream(,$pngBytesValue); \
           $bitmap = [System.Drawing.Bitmap]::FromStream($ms); \
           $pngStream = New-Object System.IO.MemoryStream; \
           $bitmap.Save($pngStream, [System.Drawing.Imaging.ImageFormat]::Png); \
           $pngBase64 = [Convert]::ToBase64String($pngStream.ToArray()); \
           $width = $bitmap.Width; \
           $height = $bitmap.Height; \
           $bitmap.Dispose(); \
           $pngStream.Dispose(); \
           $ms.Dispose(); \
         } elseif ($data.GetDataPresent([System.Windows.Forms.DataFormats]::FileDrop)) { \
           $files = $data.GetData([System.Windows.Forms.DataFormats]::FileDrop); \
           if ($files -is [string[]]) { \
             $imageFile = $files | Where-Object { $_ -match '\\.(png|jpg|jpeg|gif|bmp|webp)$' } | Select-Object -First 1; \
             if (-not [string]::IsNullOrWhiteSpace($imageFile) -and (Test-Path $imageFile)) { \
               $bitmap = [System.Drawing.Bitmap]::FromFile($imageFile); \
               $pngStream = New-Object System.IO.MemoryStream; \
               $bitmap.Save($pngStream, [System.Drawing.Imaging.ImageFormat]::Png); \
               $pngBase64 = [Convert]::ToBase64String($pngStream.ToArray()); \
               $width = $bitmap.Width; \
               $height = $bitmap.Height; \
               $bitmap.Dispose(); \
               $pngStream.Dispose(); \
             } \
           } \
         } elseif (-not [string]::IsNullOrEmpty($html)) { \
           $inlineMatch = [regex]::Match($html, 'src=\"data:image/[^;]+;base64,([^\"'']+)\"', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase); \
           if ($inlineMatch.Success) { \
             $bytes = [Convert]::FromBase64String($inlineMatch.Groups[1].Value); \
             $ms = New-Object System.IO.MemoryStream(,$bytes); \
             $bitmap = [System.Drawing.Bitmap]::FromStream($ms); \
             $pngStream = New-Object System.IO.MemoryStream; \
             $bitmap.Save($pngStream, [System.Drawing.Imaging.ImageFormat]::Png); \
             $pngBase64 = [Convert]::ToBase64String($pngStream.ToArray()); \
             $width = $bitmap.Width; \
             $height = $bitmap.Height; \
             $bitmap.Dispose(); \
             $pngStream.Dispose(); \
             $ms.Dispose(); \
           } else { \
             $fileMatch = [regex]::Match($html, 'file:///([^\"''>]+?\\.(png|jpg|jpeg|gif|bmp|webp))', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase); \
             if ($fileMatch.Success) { \
               $filePath = $fileMatch.Groups[1].Value -replace '/', '\\'; \
               $filePath = [System.Uri]::UnescapeDataString($filePath); \
               if (Test-Path $filePath) { \
                 $bitmap = [System.Drawing.Bitmap]::FromFile($filePath); \
                 $pngStream = New-Object System.IO.MemoryStream; \
                 $bitmap.Save($pngStream, [System.Drawing.Imaging.ImageFormat]::Png); \
                 $pngBase64 = [Convert]::ToBase64String($pngStream.ToArray()); \
                 $width = $bitmap.Width; \
                 $height = $bitmap.Height; \
                 $bitmap.Dispose(); \
                 $pngStream.Dispose(); \
               } \
             } \
           } \
         } \
         if ([string]::IsNullOrEmpty($text) -and [string]::IsNullOrEmpty($html) -and [string]::IsNullOrEmpty($rtf) -and $null -eq $pngBase64) { return }; \
         $payload = @{ \
           textBase64 = if ([string]::IsNullOrEmpty($text)) { $null } else { [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($text)) }; \
           htmlBase64 = if ([string]::IsNullOrEmpty($html)) { $null } else { [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($html)) }; \
           rtfBase64 = if ([string]::IsNullOrEmpty($rtf)) { $null } else { [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($rtf)) }; \
           pngBase64 = $pngBase64; \
           width = $width; \
           height = $height \
         } | ConvertTo-Json -Compress; \
         Write-Output $payload",
    )? else {
        return Ok(None);
    };

    if output.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(serde_json::from_str(&output)?))
}

// The active foreground app is stored with each entry so users can see where content came from.
#[cfg(windows)]
pub(crate) fn capture_foreground_app() -> Result<Option<ForegroundAppResult>> {
    let output = crate::powershell(
        "$ErrorActionPreference='Stop'; \
         [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
         Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class PowerPasteWin32 { [DllImport(\"user32.dll\")] public static extern IntPtr GetForegroundWindow(); [DllImport(\"user32.dll\")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId); }'; \
         $hwnd = [PowerPasteWin32]::GetForegroundWindow(); \
         if ($hwnd -eq [IntPtr]::Zero) { return }; \
         $processIdValue = 0; \
         [void][PowerPasteWin32]::GetWindowThreadProcessId($hwnd, [ref]$processIdValue); \
         if ($processIdValue -le 0) { return }; \
         $process = Get-Process -Id $processIdValue -ErrorAction SilentlyContinue; \
         if ($null -eq $process) { return }; \
         $name = $process.ProcessName; \
         $title = $process.MainWindowTitle; \
         if ([string]::IsNullOrWhiteSpace($title)) { \
           try { $title = (Get-Process -Id $processIdValue -ErrorAction Stop).MainWindowTitle } catch { $title = '' } \
         } \
         if ([string]::IsNullOrWhiteSpace($title)) { \
           try { $title = (Get-CimInstance Win32_Process -Filter \"ProcessId = $processIdValue\" -ErrorAction Stop).Name } catch { $title = '' } \
         } \
         $iconBase64 = $null; \
         try { \
           $processPath = $process.Path; \
           if ([string]::IsNullOrWhiteSpace($processPath)) { \
             $processPath = (Get-CimInstance Win32_Process -Filter \"ProcessId = $processIdValue\" -ErrorAction Stop).ExecutablePath \
           } \
           if (-not [string]::IsNullOrWhiteSpace($processPath)) { \
             Add-Type -AssemblyName System.Drawing; \
             $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($processPath); \
             if ($null -ne $icon) { \
               $bitmap = $icon.ToBitmap(); \
               $ms = New-Object System.IO.MemoryStream; \
               $bitmap.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png); \
               $iconBase64 = [Convert]::ToBase64String($ms.ToArray()); \
               $bitmap.Dispose(); \
               $icon.Dispose(); \
               $ms.Dispose(); \
             } \
           } \
         } catch { } \
         @{ processName = $name; displayName = $title; iconPngBase64 = $iconBase64 } | ConvertTo-Json -Compress",
    )?;

    if output.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(serde_json::from_str(&output)?))
}

#[cfg(not(windows))]
pub(crate) fn capture_foreground_app() -> Result<Option<ForegroundAppResult>> {
    #[cfg(target_os = "macos")]
    {
        let Some(front) = run_macos_command("lsappinfo", &["front"])? else {
            return Ok(None);
        };
        let front = front.trim_end_matches(':');
        let Some(info) =
            run_macos_command("lsappinfo", &["info", "-only", "bundlepath,name", front])?
        else {
            return Ok(None);
        };
        let display_name = parse_lsappinfo_field(&info, "LSDisplayName").unwrap_or_default();
        let app_path = parse_lsappinfo_field(&info, "LSBundlePath");
        let process_name = app_path
            .as_deref()
            .and_then(|path| std::path::Path::new(path).file_stem())
            .and_then(|stem| stem.to_str())
            .unwrap_or(display_name.as_str())
            .to_string();

        if !display_name.is_empty() || !process_name.is_empty() {
            return Ok(Some(ForegroundAppResult {
                process_name,
                display_name,
                icon_png_base64: None,
                app_path,
            }));
        }
    }

    Ok(None)
}

fn should_ignore_app(settings: &AppSettings, app: Option<&ForegroundAppResult>) -> bool {
    let Some(app) = app else {
        return false;
    };

    let process_name = app.process_name.to_lowercase();
    let display_name = app.display_name.to_lowercase();

    settings.ignored_apps.iter().any(|ignored| {
        let ignored = ignored.trim().to_lowercase();
        !ignored.is_empty() && (process_name.contains(&ignored) || display_name.contains(&ignored))
    })
}

fn is_image_placeholder_text(text: &str) -> bool {
    let normalized = text.trim().to_lowercase();
    matches!(
        normalized.as_str(),
        "[é¥å‰§å¢–]" | "é¥å‰§å¢–" | "[image]" | "image" | "[img]" | "img"
    )
}

// Capture prefers the richest payload available, but still falls back cleanly when images are too large.
#[cfg(windows)]
pub(crate) fn capture_clipboard(
    settings: &AppSettings,
    source_app: Option<&ForegroundAppResult>,
) -> Result<Option<CapturedClipboard>> {
    if should_ignore_app(settings, source_app) {
        return Ok(None);
    }

    let Some(payload) = capture_clipboard_payload()? else {
        return Ok(None);
    };

    let decode = |value: Option<String>| -> Result<Option<String>> {
        value
            .map(|encoded| {
                let bytes = BASE64.decode(encoded.as_bytes())?;
                Ok(String::from_utf8(bytes)?)
            })
            .transpose()
    };

    let text = decode(payload.text_base64)?.unwrap_or_default();
    let html_text = decode(payload.html_base64)?;
    let rtf_text = decode(payload.rtf_base64)?;
    let png_bytes = payload
        .png_base64
        .map(|encoded| BASE64.decode(encoded.as_bytes()))
        .transpose()?;

    let rich_text_is_empty = html_text
        .as_deref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
        && rtf_text
            .as_deref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true);

    if let Some(png_bytes) = png_bytes.as_ref() {
        let normalized = text.trim();
        if (normalized == "[é¥å‰§å¢–]"
            || normalized == "é¥å‰§å¢–"
            || is_image_placeholder_text(&text))
            && rich_text_is_empty
        {
            if png_bytes.len() > settings.max_image_bytes {
                return Ok(None);
            }
            let image_hash = image_hash_from_png_bytes(png_bytes)?;
            return Ok(Some(CapturedClipboard::Image {
                hash: image_hash,
                preview: format!(
                    "Image {}x{}",
                    payload.width.unwrap_or_default(),
                    payload.height.unwrap_or_default()
                ),
                png_bytes: png_bytes.clone(),
                image_width: payload.width.unwrap_or_default(),
                image_height: payload.height.unwrap_or_default(),
            }));
        }
    }

    if !text.is_empty() && png_bytes.is_some() {
        let png_bytes = png_bytes.unwrap_or_default();
        if png_bytes.len() > settings.max_image_bytes {
            return Ok(Some(CapturedClipboard::Text {
                hash: text_hash(&text, html_text.as_deref(), rtf_text.as_deref()),
                text,
                html_text,
                rtf_text,
            }));
        }
        let hash = mixed_hash(&text, &png_bytes)?;
        return Ok(Some(CapturedClipboard::Mixed {
            text,
            html_text,
            rtf_text,
            png_bytes,
            hash,
            image_width: payload.width.unwrap_or_default(),
            image_height: payload.height.unwrap_or_default(),
        }));
    }

    if !text.is_empty() || html_text.as_deref().is_some() || rtf_text.as_deref().is_some() {
        return Ok(Some(CapturedClipboard::Text {
            hash: text_hash(&text, html_text.as_deref(), rtf_text.as_deref()),
            text,
            html_text,
            rtf_text,
        }));
    }

    if let Some(png_bytes) = png_bytes {
        if png_bytes.len() > settings.max_image_bytes {
            return Ok(None);
        }
        let image_hash = image_hash_from_png_bytes(&png_bytes)?;
        return Ok(Some(CapturedClipboard::Image {
            hash: image_hash,
            preview: format!(
                "Image {}x{}",
                payload.width.unwrap_or_default(),
                payload.height.unwrap_or_default()
            ),
            png_bytes,
            image_width: payload.width.unwrap_or_default(),
            image_height: payload.height.unwrap_or_default(),
        }));
    }

    Ok(None)
}

#[cfg(not(windows))]
pub(crate) fn capture_clipboard(
    settings: &AppSettings,
    source_app: Option<&ForegroundAppResult>,
) -> Result<Option<CapturedClipboard>> {
    #[cfg(target_os = "macos")]
    {
        if should_ignore_app(settings, source_app) {
            return Ok(None);
        }

        let Some(text) = run_macos_command("pbpaste", &[])?
            .map(|value| value.replace("\r\n", "\n").replace('\r', "\n"))
            .filter(|value| !value.is_empty())
        else {
            return Ok(None);
        };

        return Ok(Some(CapturedClipboard::Text {
            hash: text_hash(&text, None, None),
            text,
            html_text: None,
            rtf_text: None,
        }));
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = settings;
        let _ = source_app;
        Ok(None)
    }
}

// Deduplicates repeated captures, persists image assets, and enforces history size limits.
pub(crate) fn store_capture(
    paths: &StoragePaths,
    history: &mut Vec<StoredClipboardItem>,
    capture: CapturedClipboard,
    source_app: Option<(String, Option<String>)>,
    settings: &AppSettings,
) -> Result<bool> {
    let hash = match &capture {
        CapturedClipboard::Text { hash, .. }
        | CapturedClipboard::Image { hash, .. }
        | CapturedClipboard::Mixed { hash, .. } => hash.as_str(),
    };

    let matching_text = match &capture {
        CapturedClipboard::Text { text, .. } | CapturedClipboard::Mixed { text, .. }
            if !text.is_empty() =>
        {
            Some(text.as_str())
        }
        _ => None,
    };

    if let Some(existing) = history.iter_mut().find(|item| {
        item.hash == hash
            || (item.kind == "text"
                && matching_text
                    .map(|text| item.full_text.as_deref() == Some(text))
                    .unwrap_or(false))
    }) {
        existing.created_at = Utc::now().to_rfc3339();
        existing.source_app = source_app.as_ref().map(|app| app.0.clone());
        existing.source_icon_data_url = source_app.as_ref().and_then(|app| app.1.clone());
        existing.hash = hash.to_string();
        existing.preview = match &capture {
            CapturedClipboard::Text {
                text,
                html_text,
                rtf_text,
                ..
            } => {
                existing.full_text = Some(text.clone());
                existing.html_text = html_text.clone();
                existing.rtf_text = rtf_text.clone();
                existing.image_path = None;
                existing.image_data_url = None;
                existing.image_width = None;
                existing.image_height = None;
                existing.kind = "text".into();
                preview_text(text)
            }
            CapturedClipboard::Image { preview, .. } => preview.clone(),
            CapturedClipboard::Mixed {
                text,
                html_text,
                rtf_text,
                png_bytes,
                image_width,
                image_height,
                ..
            } => {
                let image_path = paths.image_dir.join(format!("{}.png", existing.id));
                fs::write(&image_path, png_bytes)?;
                existing.kind = "mixed".into();
                existing.full_text = Some(text.clone());
                existing.html_text = html_text.clone();
                existing.rtf_text = rtf_text.clone();
                existing.image_path = Some(image_path.to_string_lossy().to_string());
                existing.image_data_url = Some(format!(
                    "data:image/png;base64,{}",
                    BASE64.encode(png_bytes)
                ));
                existing.image_width = Some(*image_width);
                existing.image_height = Some(*image_height);
                preview_text(text)
            }
        };
        history.sort_by(sort_history);
        save_history(paths, history)?;
        return Ok(false);
    }

    let mut item = StoredClipboardItem {
        id: Uuid::new_v4().to_string(),
        kind: String::new(),
        created_at: Utc::now().to_rfc3339(),
        pinned_at: None,
        preview: String::new(),
        full_text: None,
        html_text: None,
        rtf_text: None,
        image_path: None,
        image_data_url: None,
        image_width: None,
        image_height: None,
        source_app: source_app.as_ref().map(|app| app.0.clone()),
        source_icon_data_url: source_app.and_then(|app| app.1),
        hash: hash.to_string(),
        pinned: false,
        favorite: false,
    };

    match capture {
        CapturedClipboard::Text {
            text,
            html_text,
            rtf_text,
            ..
        } => {
            item.kind = "text".into();
            item.preview = preview_text(&text);
            item.full_text = Some(text);
            item.html_text = html_text;
            item.rtf_text = rtf_text;
        }
        CapturedClipboard::Image {
            png_bytes,
            preview,
            image_width,
            image_height,
            ..
        } => {
            let image_path = paths.image_dir.join(format!("{}.png", item.id));
            fs::write(&image_path, &png_bytes)?;
            item.kind = "image".into();
            item.preview = preview;
            item.image_path = Some(image_path.to_string_lossy().to_string());
            item.image_data_url = Some(format!(
                "data:image/png;base64,{}",
                BASE64.encode(&png_bytes)
            ));
            item.image_width = Some(image_width);
            item.image_height = Some(image_height);
        }
        CapturedClipboard::Mixed {
            text,
            html_text,
            rtf_text,
            png_bytes,
            image_width,
            image_height,
            ..
        } => {
            let image_path = paths.image_dir.join(format!("{}.png", item.id));
            fs::write(&image_path, &png_bytes)?;
            item.kind = "mixed".into();
            item.preview = preview_text(&text);
            item.full_text = Some(text);
            item.html_text = html_text;
            item.rtf_text = rtf_text;
            item.image_path = Some(image_path.to_string_lossy().to_string());
            item.image_data_url = Some(format!(
                "data:image/png;base64,{}",
                BASE64.encode(&png_bytes)
            ));
            item.image_width = Some(image_width);
            item.image_height = Some(image_height);
        }
    }

    history.push(item);
    history.sort_by(sort_history);

    while history.len() > settings.max_history_items {
        if let Some(index) = history.iter().rposition(|item| !item.pinned) {
            if let Some(image_path) = history[index].image_path.clone() {
                let _ = fs::remove_file(image_path);
            }
            history.remove(index);
        } else {
            break;
        }
    }

    save_history(paths, history)?;
    Ok(true)
}

// Pinned items sort first, then favorites, then most recent updates.
pub(crate) fn sort_history(
    left: &StoredClipboardItem,
    right: &StoredClipboardItem,
) -> std::cmp::Ordering {
    right
        .pinned
        .cmp(&left.pinned)
        .then(right.pinned_at.cmp(&left.pinned_at))
        .then(right.favorite.cmp(&left.favorite))
        .then(right.created_at.cmp(&left.created_at))
}

// Search is intentionally simple: preview, full text and source app all participate in matching.
pub(crate) fn history_to_dto(
    items: &[StoredClipboardItem],
    query: Option<&str>,
    limit: usize,
) -> Vec<ClipboardItemDto> {
    let needle = query.unwrap_or("").trim().to_lowercase();

    items
        .iter()
        .filter(|item| {
            if needle.is_empty() {
                return true;
            }

            let haystack = format!(
                "{}\n{}\n{}",
                item.preview,
                item.full_text.clone().unwrap_or_default(),
                item.source_app.clone().unwrap_or_default()
            )
            .to_lowercase();

            haystack.contains(&needle)
        })
        .take(limit)
        .map(|item| ClipboardItemDto {
            id: item.id.clone(),
            kind: item.kind.clone(),
            created_at: item.created_at.clone(),
            preview: item.preview.clone(),
            full_text: item.full_text.clone(),
            image_data_url: item.image_data_url.clone(),
            image_width: item.image_width,
            image_height: item.image_height,
            source_app: item.source_app.clone(),
            source_icon_data_url: item.source_icon_data_url.clone(),
            pinned: item.pinned,
            favorite: item.favorite,
        })
        .collect()
}
