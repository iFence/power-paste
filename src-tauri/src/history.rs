use anyhow::Result;
#[cfg(target_os = "macos")]
use std::collections::HashMap;
#[cfg(target_os = "macos")]
use std::sync::{Mutex, OnceLock};

use crate::{
    models::{
        AppSettings, CapturedClipboard, ClipboardItemDto, ForegroundAppResult, StoredClipboardItem,
    },
    repository::SqliteHistoryStore,
    storage::{image_hash_from_png_bytes, mixed_hash, text_hash},
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

fn normalized_app_display_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("Program Manager") {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn source_app_label(app: ForegroundAppResult) -> Option<String> {
    if let Some(display_name) = normalized_app_display_name(&app.display_name) {
        return Some(display_name);
    }

    let process_name = app.process_name.trim();
    if process_name.is_empty() {
        None
    } else {
        Some(friendly_process_name(process_name))
    }
}

pub(crate) fn normalize_link_url(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_whitespace) {
        return None;
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Some(trimmed.to_string());
    }

    if trimmed.starts_with("www.") {
        return Some(format!("https://{trimmed}"));
    }

    None
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
         $processPath = $null; \
         try { \
           $processPath = $process.Path; \
         } catch { } \
         if ([string]::IsNullOrWhiteSpace($processPath)) { \
           try { \
             $processPath = (Get-CimInstance Win32_Process -Filter \"ProcessId = $processIdValue\" -ErrorAction Stop).ExecutablePath \
           } catch { } \
         } \
         $appDisplayName = $null; \
         if (-not [string]::IsNullOrWhiteSpace($processPath)) { \
           try { \
             $versionInfo = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($processPath); \
             foreach ($candidate in @($versionInfo.ProductName, $versionInfo.FileDescription, $versionInfo.InternalName)) { \
               if (-not [string]::IsNullOrWhiteSpace($candidate)) { \
                 $appDisplayName = $candidate; \
                 break; \
               } \
             } \
           } catch { } \
         } \
         if ([string]::IsNullOrWhiteSpace($appDisplayName)) { \
           $appDisplayName = $name; \
         } \
         $iconBase64 = $null; \
         try { \
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
         @{ processName = $name; displayName = $appDisplayName; iconPngBase64 = $iconBase64; appPath = $processPath } | ConvertTo-Json -Compress",
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

pub(crate) fn should_ignore_app(settings: &AppSettings, app: Option<&ForegroundAppResult>) -> bool {
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

pub(crate) fn is_image_placeholder_text(text: &str) -> bool {
    let normalized = text.trim().to_lowercase();
    matches!(normalized.as_str(), "[image]" | "image" | "[img]" | "img")
}

pub(crate) fn build_captured_clipboard(
    settings: &AppSettings,
    text: String,
    html_text: Option<String>,
    rtf_text: Option<String>,
    png_bytes: Option<Vec<u8>>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<Option<CapturedClipboard>> {
    let has_text_payload = !text.is_empty()
        || html_text
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        || rtf_text
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
    let rich_text_is_empty = html_text
        .as_deref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
        && rtf_text
            .as_deref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true);

    if let Some(png_bytes) = png_bytes.as_ref() {
        if is_image_placeholder_text(&text) && rich_text_is_empty {
            if png_bytes.len() > settings.max_image_bytes {
                return Ok(None);
            }
            let image_hash = image_hash_from_png_bytes(png_bytes)?;
            return Ok(Some(CapturedClipboard::Image {
                hash: image_hash,
                preview: format!("Image {}x{}", width.unwrap_or_default(), height.unwrap_or_default()),
                png_bytes: png_bytes.clone(),
                image_width: width.unwrap_or_default(),
                image_height: height.unwrap_or_default(),
            }));
        }
    }

    if has_text_payload && png_bytes.is_some() {
        let png_bytes = png_bytes.unwrap_or_default();
        if png_bytes.len() > settings.max_image_bytes {
            return Ok(Some(CapturedClipboard::Text {
                hash: text_hash(&text, html_text.as_deref(), rtf_text.as_deref()),
                text,
                html_text,
                rtf_text,
            }));
        }
        let hash = mixed_hash(&text, html_text.as_deref(), rtf_text.as_deref(), &png_bytes)?;
        return Ok(Some(CapturedClipboard::Mixed {
            text,
            html_text,
            rtf_text,
            png_bytes,
            hash,
            image_width: width.unwrap_or_default(),
            image_height: height.unwrap_or_default(),
        }));
    }

    if !text.is_empty() || html_text.as_deref().is_some() || rtf_text.as_deref().is_some() {
        let hash = text_hash(&text, html_text.as_deref(), rtf_text.as_deref());
        if normalize_link_url(&text).is_some() {
            return Ok(Some(CapturedClipboard::Link {
                hash,
                text,
                html_text,
                rtf_text,
            }));
        }

        return Ok(Some(CapturedClipboard::Text {
            hash,
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
            preview: format!("Image {}x{}", width.unwrap_or_default(), height.unwrap_or_default()),
            png_bytes,
            image_width: width.unwrap_or_default(),
            image_height: height.unwrap_or_default(),
        }));
    }

    Ok(None)
}

pub(crate) fn store_capture(
    store: &mut SqliteHistoryStore,
    history: &mut Vec<StoredClipboardItem>,
    capture: CapturedClipboard,
    source_app: Option<(String, Option<String>)>,
    settings: &AppSettings,
) -> Result<bool> {
    let inserted = store.upsert_capture(capture, source_app, settings)?;
    *history = store.list_all()?;
    Ok(inserted)
}

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
        .map(history_item_to_dto)
        .collect()
}

pub(crate) fn history_item_to_dto(item: &StoredClipboardItem) -> ClipboardItemDto {
    ClipboardItemDto {
        id: item.id.clone(),
        kind: item.kind.clone(),
        created_at: item.created_at.clone(),
        preview: item.preview.clone(),
        full_text: item.full_text.clone(),
        image_data_url: item.image_data_url(),
        image_byte_size: item.image_png.as_ref().map(|bytes| bytes.len()),
        image_width: item.image_width,
        image_height: item.image_height,
        source_app: item.source_app.clone(),
        source_icon_data_url: item.source_icon_data_url.clone(),
        pinned: item.pinned,
        favorite: item.favorite,
    }
}

#[cfg(test)]
mod tests {
    use super::{build_captured_clipboard, normalized_app_display_name, source_app_label};
    use crate::models::{AppSettings, CapturedClipboard, ForegroundAppResult};
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

    #[test]
    fn prefers_stable_display_name_for_source_app_label() {
        let label = source_app_label(ForegroundAppResult {
            process_name: "pixpin".into(),
            display_name: "PixPin".into(),
            icon_png_base64: None,
            app_path: Some("C:\\Program Files\\PixPin\\PixPin.exe".into()),
        });

        assert_eq!(label.as_deref(), Some("PixPin"));
    }

    #[test]
    fn falls_back_to_process_name_when_display_name_is_not_usable() {
        let label = source_app_label(ForegroundAppResult {
            process_name: "dingtalk".into(),
            display_name: "Program Manager".into(),
            icon_png_base64: None,
            app_path: None,
        });

        assert_eq!(label.as_deref(), Some("Dingtalk"));
    }

    #[test]
    fn rejects_empty_display_name() {
        assert_eq!(normalized_app_display_name("   "), None);
    }

    #[test]
    fn classifies_html_plus_image_as_mixed() {
        let settings = AppSettings::default();
        let mut png_bytes = Vec::new();
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255])))
            .write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)
            .expect("png");

        let capture = build_captured_clipboard(
            &settings,
            String::new(),
            Some("<p>hello</p><img src=\"x\" />".into()),
            None,
            Some(png_bytes),
            Some(1),
            Some(1),
        )
        .expect("capture")
        .expect("mixed");

        assert!(matches!(capture, CapturedClipboard::Mixed { .. }));
    }
}
