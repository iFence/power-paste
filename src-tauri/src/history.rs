use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
#[cfg(target_os = "macos")]
use std::collections::HashMap;
#[cfg(windows)]
use std::ffi::c_void;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "macos")]
use std::sync::{Mutex, OnceLock};

use crate::{
    models::{
        AppSettings, CapturedClipboard, ClipboardItemDto, ForegroundAppResult, IgnoredAppRule,
        StoredClipboardItem,
    },
    repository::SqliteHistoryStore,
    rich_text::{first_html_image_src, html_contains_image_content, normalize_rich_text_payload},
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

#[cfg(target_os = "linux")]
fn run_linux_command(program: &str, args: &[&str]) -> Result<Option<String>> {
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

#[cfg(target_os = "linux")]
fn linux_window_display_name(window_id: &str) -> Option<String> {
    run_linux_command("xdotool", &["getwindowclassname", window_id])
        .ok()
        .flatten()
        .or_else(|| {
            run_linux_command("xdotool", &["getwindowname", window_id])
                .ok()
                .flatten()
        })
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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

#[cfg(windows)]
fn windows_app_icon_base64(app_path: &str) -> Option<String> {
    use image::{DynamicImage, ImageFormat, RgbaImage};
    use windows_sys::Win32::{
        Graphics::Gdi::{
            CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetObjectW, SelectObject,
            BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
        },
        UI::{
            Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON},
            WindowsAndMessaging::{DestroyIcon, DrawIconEx, GetIconInfo, DI_NORMAL, ICONINFO},
        },
    };

    fn encode_icon_bitmap_to_base64(
        icon_handle: windows_sys::Win32::UI::WindowsAndMessaging::HICON,
        width: i32,
        height: i32,
    ) -> Option<String> {
        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
        let mut bits_ptr: *mut c_void = std::ptr::null_mut();
        let bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            },
            ..Default::default()
        };

        unsafe {
            let dc = CreateCompatibleDC(std::ptr::null_mut());
            if dc.is_null() {
                return None;
            }

            let bitmap = CreateDIBSection(
                dc,
                &bitmap_info,
                DIB_RGB_COLORS,
                &mut bits_ptr,
                std::ptr::null_mut(),
                0,
            );
            if bitmap.is_null() || bits_ptr.is_null() {
                DeleteDC(dc);
                return None;
            }

            let old_object = SelectObject(dc, bitmap as HGDIOBJ);
            let draw_ok = DrawIconEx(
                dc,
                0,
                0,
                icon_handle,
                width,
                height,
                0,
                std::ptr::null_mut(),
                DI_NORMAL,
            ) != 0;

            if draw_ok {
                std::ptr::copy_nonoverlapping(
                    bits_ptr as *const u8,
                    pixels.as_mut_ptr(),
                    pixels.len(),
                );
            }

            SelectObject(dc, old_object);
            DeleteObject(bitmap as HGDIOBJ);
            DeleteDC(dc);

            if !draw_ok {
                return None;
            }
        }

        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        let image = RgbaImage::from_raw(width as u32, height as u32, pixels)?;
        let mut png_bytes = Vec::new();
        DynamicImage::ImageRgba8(image)
            .write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)
            .ok()?;

        Some(BASE64.encode(png_bytes))
    }

    let wide_path: Vec<u16> = app_path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut file_info = SHFILEINFOW::default();
    let icon_handle = unsafe {
        let result = SHGetFileInfoW(
            wide_path.as_ptr(),
            0,
            &mut file_info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if result == 0 {
            return None;
        }
        file_info.hIcon
    };
    if icon_handle.is_null() {
        return None;
    }

    let mut icon_info = ICONINFO::default();
    let extracted = unsafe { GetIconInfo(icon_handle, &mut icon_info) != 0 };
    if !extracted {
        unsafe {
            DestroyIcon(icon_handle);
        }
        return None;
    }

    let mut bitmap = BITMAP::default();
    let bitmap_handle = if !icon_info.hbmColor.is_null() {
        icon_info.hbmColor
    } else {
        icon_info.hbmMask
    };
    let got_bitmap = unsafe {
        GetObjectW(
            bitmap_handle as *mut c_void,
            std::mem::size_of::<BITMAP>() as i32,
            &mut bitmap as *mut BITMAP as *mut c_void,
        ) != 0
    };

    let result = if got_bitmap {
        let width = bitmap.bmWidth.max(1);
        let height = if !icon_info.hbmColor.is_null() {
            bitmap.bmHeight.max(1)
        } else {
            (bitmap.bmHeight / 2).max(1)
        };
        encode_icon_bitmap_to_base64(icon_handle, width, height)
    } else {
        None
    };

    unsafe {
        if !icon_info.hbmColor.is_null() {
            DeleteObject(icon_info.hbmColor as HGDIOBJ);
        }
        if !icon_info.hbmMask.is_null() {
            DeleteObject(icon_info.hbmMask as HGDIOBJ);
        }
        DestroyIcon(icon_handle);
    }

    result
}

pub(crate) fn source_app_icon_data_url(app: &ForegroundAppResult) -> Option<String> {
    let icon_base64 = app
        .icon_png_base64
        .clone()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            #[cfg(windows)]
            {
                app.app_path.as_deref().and_then(windows_app_icon_base64)
            }
            #[cfg(target_os = "macos")]
            {
                app.app_path.as_deref().and_then(macos_app_icon_base64)
            }
            #[cfg(all(not(windows), not(target_os = "macos")))]
            {
                None
            }
        })?;

    Some(format!("data:image/png;base64,{icon_base64}"))
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
        bundle_id: app.bundle_id.clone(),
    })?;
    let icon = app
        .icon_png_base64
        .filter(|value| !value.is_empty())
        .map(|value| format!("data:image/png;base64,{value}"));
    Some((label, icon))
}

#[cfg(windows)]
pub(crate) fn capture_foreground_app() -> Result<Option<ForegroundAppResult>> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    };

    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() {
        return Ok(None);
    }

    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut process_id);
    }
    if process_id == 0 {
        return Ok(None);
    }

    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if process.is_null() {
        return Ok(None);
    }

    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    let path = unsafe {
        let ok = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) != 0;
        CloseHandle(process);
        if ok {
            Some(String::from_utf16_lossy(&buffer[..length as usize]))
        } else {
            None
        }
    };

    let process_name = path
        .as_deref()
        .and_then(|value| std::path::Path::new(value).file_stem())
        .and_then(|value| value.to_str())
        .map(ToString::to_string)
        .unwrap_or_default();

    if process_name.is_empty() {
        return Ok(None);
    }

    Ok(Some(ForegroundAppResult {
        display_name: friendly_process_name(&process_name),
        process_name,
        icon_png_base64: None,
        app_path: path,
        bundle_id: None,
    }))
}

#[cfg(not(windows))]
pub(crate) fn capture_foreground_app() -> Result<Option<ForegroundAppResult>> {
    #[cfg(target_os = "macos")]
    {
        let Some(front) = run_macos_command("lsappinfo", &["front"])? else {
            return Ok(None);
        };
        let front = front.trim_end_matches(':');
        let Some(info) = run_macos_command(
            "lsappinfo",
            &["info", "-only", "bundleid,bundlepath,name", front],
        )?
        else {
            return Ok(None);
        };
        let display_name = parse_lsappinfo_field(&info, "LSDisplayName").unwrap_or_default();
        let app_path = parse_lsappinfo_field(&info, "LSBundlePath");
        let bundle_id = parse_lsappinfo_field(&info, "LSBundleIdentifier");
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
                bundle_id,
            }));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if crate::clipboard::linux_session_backend() != "x11"
            || !crate::clipboard::linux_x11_tooling_available()
        {
            return Ok(None);
        }

        let Some(window_id) = run_linux_command("xdotool", &["getactivewindow"])? else {
            return Ok(None);
        };
        let pid = run_linux_command("xdotool", &["getwindowpid", window_id.as_str()])?;
        let app_path = pid.as_deref().and_then(|value| {
            fs::read_link(format!("/proc/{value}/exe"))
                .ok()
                .map(|path| path.to_string_lossy().to_string())
        });
        let process_name = pid
            .as_deref()
            .and_then(|value| fs::read_to_string(format!("/proc/{value}/comm")).ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or_else(|| {
                app_path
                    .as_deref()
                    .and_then(|path| std::path::Path::new(path).file_stem())
                    .and_then(|stem| stem.to_str())
                    .map(ToString::to_string)
            })
            .or_else(|| linux_window_display_name(window_id.as_str()))
            .unwrap_or_default();
        let display_name =
            linux_window_display_name(window_id.as_str()).unwrap_or_else(|| process_name.clone());

        if !display_name.is_empty() || !process_name.is_empty() {
            return Ok(Some(ForegroundAppResult {
                process_name,
                display_name,
                icon_png_base64: None,
                app_path,
                bundle_id: None,
            }));
        }
    }

    Ok(None)
}

fn current_platform_key() -> &'static str {
    #[cfg(windows)]
    {
        "windows"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        "unknown"
    }
}

fn normalize_app_match_value(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_app_path_match(value: &str) -> String {
    value.trim().replace('\\', "/").to_ascii_lowercase()
}

fn rule_matches_app(rule: &IgnoredAppRule, app: &ForegroundAppResult) -> bool {
    if !rule.enabled {
        return false;
    }
    if !rule.platform.is_empty() && rule.platform != current_platform_key() {
        return false;
    }

    let app_path = app.app_path.as_deref().map(normalize_app_path_match);
    if let Some(rule_path) = rule.app_path.as_deref().map(normalize_app_path_match) {
        return app_path.as_deref() == Some(rule_path.as_str());
    }

    let bundle_id = app.bundle_id.as_deref().map(normalize_app_match_value);
    if let Some(rule_bundle_id) = rule.bundle_id.as_deref().map(normalize_app_match_value) {
        return bundle_id.as_deref() == Some(rule_bundle_id.as_str());
    }

    let process_name = normalize_app_match_value(&app.process_name);
    if !rule.process_name.is_empty()
        && process_name == normalize_app_match_value(&rule.process_name)
    {
        return true;
    }

    let display_name = normalize_app_match_value(&app.display_name);
    !rule.display_name.is_empty() && display_name == normalize_app_match_value(&rule.display_name)
}

pub(crate) fn should_ignore_app(settings: &AppSettings, app: Option<&ForegroundAppResult>) -> bool {
    let Some(app) = app else {
        return false;
    };

    settings
        .ignored_app_rules
        .iter()
        .any(|rule| rule_matches_app(rule, app))
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
    original_bytes: Option<Vec<u8>>,
    original_mime: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<Option<CapturedClipboard>> {
    let (text, html_text) = normalize_rich_text_payload(Some(text), html_text);
    let text = text.unwrap_or_default();

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
    let html_has_image_content = html_text
        .as_deref()
        .map(html_contains_image_content)
        .unwrap_or(false);

    if let Some(png_bytes) = png_bytes.as_ref() {
        if is_image_placeholder_text(&text) && rich_text_is_empty {
            if png_bytes.len() > settings.max_image_bytes {
                return Ok(None);
            }
            let image_hash =
                image_hash_from_png_bytes(original_bytes.as_deref().unwrap_or(png_bytes))?;
            return Ok(Some(CapturedClipboard::Image {
                hash: image_hash,
                preview: format!(
                    "Image {}x{}",
                    width.unwrap_or_default(),
                    height.unwrap_or_default()
                ),
                png_bytes: png_bytes.clone(),
                original_bytes: original_bytes.clone(),
                original_mime: original_mime.clone(),
                image_width: width.unwrap_or_default(),
                image_height: height.unwrap_or_default(),
            }));
        }
    }

    if has_text_payload && (png_bytes.is_some() || html_has_image_content) {
        if let Some(bytes) = png_bytes.as_ref() {
            if bytes.len() > settings.max_image_bytes {
                return Ok(Some(CapturedClipboard::Text {
                    hash: text_hash(&text, html_text.as_deref(), rtf_text.as_deref()),
                    text,
                    html_text,
                    rtf_text,
                }));
            }
        }

        let hash = if let Some(bytes) = png_bytes.as_deref() {
            mixed_hash(&text, html_text.as_deref(), rtf_text.as_deref(), bytes)?
        } else {
            text_hash(&text, html_text.as_deref(), rtf_text.as_deref())
        };
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
        let image_hash =
            image_hash_from_png_bytes(original_bytes.as_deref().unwrap_or(png_bytes.as_slice()))?;
        return Ok(Some(CapturedClipboard::Image {
            hash: image_hash,
            preview: format!(
                "Image {}x{}",
                width.unwrap_or_default(),
                height.unwrap_or_default()
            ),
            png_bytes,
            original_bytes,
            original_mime,
            image_width: width.unwrap_or_default(),
            image_height: height.unwrap_or_default(),
        }));
    }

    Ok(None)
}

pub(crate) fn store_capture_item(
    store: &mut SqliteHistoryStore,
    capture: CapturedClipboard,
    source_app: Option<(String, Option<String>)>,
    settings: &AppSettings,
) -> Result<StoredClipboardItem> {
    let upserted = store.upsert_capture(capture, source_app, settings)?;
    let _inserted = upserted.inserted;
    Ok(upserted.item)
}

pub(crate) fn history_item_to_dto(item: &StoredClipboardItem) -> ClipboardItemDto {
    let has_image_preview = item
        .image_preview_png
        .as_ref()
        .filter(|bytes| !bytes.is_empty())
        .or(item.image_png.as_ref().filter(|bytes| !bytes.is_empty()))
        .is_some()
        || item
            .html_text
            .as_deref()
            .filter(|_| item.kind == "mixed")
            .is_some_and(html_has_supported_image_preview_source);

    ClipboardItemDto {
        id: item.id.clone(),
        kind: item.kind.clone(),
        created_at: item.created_at.clone(),
        preview: item.preview.clone(),
        full_text: item.full_text.clone(),
        image_data_url: None,
        has_image_preview,
        image_preview_url: has_image_preview
            .then(|| format!("history-preview://localhost/{}", item.id)),
        image_byte_size: item.image_display_byte_size(),
        image_width: item.image_width,
        image_height: item.image_height,
        source_app: item.source_app.clone(),
        source_icon_data_url: item.source_icon_data_url.clone(),
        pinned: item.pinned,
        favorite: item.favorite,
        tag_colors: item.tag_colors.clone(),
        copy_count: item.copy_count,
        paste_count: item.paste_count,
    }
}

pub(crate) fn html_has_supported_image_preview_source(html: &str) -> bool {
    let Some(src) = first_html_image_src(html) else {
        return false;
    };

    if src.to_ascii_lowercase().starts_with("data:image/") {
        let Some((metadata, encoded)) = src.split_once(',') else {
            return false;
        };
        return metadata.to_ascii_lowercase().ends_with(";base64") && !encoded.trim().is_empty();
    }

    local_image_path_from_src(&src).is_some()
}
pub(crate) fn html_image_preview_bytes(html: &str) -> Option<(String, Vec<u8>)> {
    let src = first_html_image_src(html)?;
    if src.to_ascii_lowercase().starts_with("data:image/") {
        return data_image_src_to_bytes(&src);
    }

    let path = local_image_path_from_src(&src)?;
    local_image_file_to_bytes(&path)
}

fn data_image_src_to_bytes(src: &str) -> Option<(String, Vec<u8>)> {
    let (metadata, encoded) = src.split_once(',')?;
    if !metadata.to_ascii_lowercase().ends_with(";base64") {
        return None;
    }
    let mime = metadata.strip_prefix("data:")?.to_string();
    let bytes = BASE64.decode(encoded).ok()?;
    (!bytes.is_empty()).then_some((mime, bytes))
}

fn local_image_path_from_src(src: &str) -> Option<std::path::PathBuf> {
    if src.to_ascii_lowercase().starts_with("file://") {
        return file_url_to_path(src);
    }

    let path = std::path::PathBuf::from(src);
    path.is_file().then_some(path)
}

fn file_url_to_path(src: &str) -> Option<std::path::PathBuf> {
    let raw = src
        .strip_prefix("file://")
        .or_else(|| src.strip_prefix("FILE://"))?;
    let normalized = raw.replace("%20", " ");

    #[cfg(windows)]
    {
        let trimmed = normalized.trim_start_matches('/');
        let path = std::path::PathBuf::from(trimmed.replace('/', "\\"));
        return path.is_file().then_some(path);
    }

    #[cfg(not(windows))]
    {
        let path = std::path::PathBuf::from(normalized);
        path.is_file().then_some(path)
    }
}

fn local_image_file_to_bytes(path: &std::path::Path) -> Option<(String, Vec<u8>)> {
    let mime = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("webp") => "image/webp",
        _ => return None,
    };

    let bytes = std::fs::read(path).ok()?;
    if bytes.is_empty() {
        return None;
    }

    Some((mime.into(), bytes))
}

#[cfg(test)]
mod tests {
    use super::{
        build_captured_clipboard, friendly_process_name, history_item_to_dto,
        normalized_app_display_name, should_ignore_app, source_app_label,
    };
    use crate::models::{
        AppSettings, CapturedClipboard, ForegroundAppResult, IgnoredAppRule, StoredClipboardItem,
    };
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
    use std::fs as std_fs;

    #[test]
    fn prefers_stable_display_name_for_source_app_label() {
        let label = source_app_label(ForegroundAppResult {
            process_name: "pixpin".into(),
            display_name: "PixPin".into(),
            icon_png_base64: None,
            app_path: Some("C:\\Program Files\\PixPin\\PixPin.exe".into()),
            bundle_id: None,
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
            bundle_id: None,
        });

        assert_eq!(label.as_deref(), Some("Dingtalk"));
    }

    #[test]
    fn formats_known_process_names_for_ui_labels() {
        assert_eq!(friendly_process_name("code"), "VS Code");
        assert_eq!(friendly_process_name("chrome"), "Google Chrome");
    }

    #[test]
    fn rejects_empty_display_name() {
        assert_eq!(normalized_app_display_name("   "), None);
    }

    fn test_app() -> ForegroundAppResult {
        ForegroundAppResult {
            process_name: "Code".into(),
            display_name: "VS Code".into(),
            icon_png_base64: None,
            app_path: Some("C:\\Program Files\\Microsoft VS Code\\Code.exe".into()),
            bundle_id: Some("com.microsoft.VSCode".into()),
        }
    }

    fn settings_with_rule(rule: IgnoredAppRule) -> AppSettings {
        let mut settings = AppSettings::default();
        settings.ignored_apps = Vec::new();
        settings.ignored_app_rules = vec![rule];
        settings.normalized()
    }

    #[test]
    fn ignores_app_by_exact_app_path() {
        let settings = settings_with_rule(IgnoredAppRule {
            app_path: Some("C:/Program Files/Microsoft VS Code/Code.exe".into()),
            ..Default::default()
        });

        assert!(should_ignore_app(&settings, Some(&test_app())));
    }

    #[test]
    fn ignores_app_by_bundle_id() {
        let settings = settings_with_rule(IgnoredAppRule {
            bundle_id: Some("com.microsoft.vscode".into()),
            ..Default::default()
        });

        assert!(should_ignore_app(&settings, Some(&test_app())));
    }

    #[test]
    fn ignores_app_by_process_name() {
        let settings = settings_with_rule(IgnoredAppRule {
            process_name: "code".into(),
            ..Default::default()
        });

        assert!(should_ignore_app(&settings, Some(&test_app())));
    }

    #[test]
    fn ignores_app_by_display_name() {
        let settings = settings_with_rule(IgnoredAppRule {
            display_name: "vs code".into(),
            ..Default::default()
        });

        assert!(should_ignore_app(&settings, Some(&test_app())));
    }

    #[test]
    fn does_not_ignore_disabled_or_partial_rules() {
        let disabled = settings_with_rule(IgnoredAppRule {
            process_name: "Code".into(),
            enabled: false,
            ..Default::default()
        });
        let partial = settings_with_rule(IgnoredAppRule {
            process_name: "Cod".into(),
            ..Default::default()
        });

        assert!(!should_ignore_app(&disabled, Some(&test_app())));
        assert!(!should_ignore_app(&partial, Some(&test_app())));
        assert!(!should_ignore_app(&partial, None));
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
            None,
            None,
            Some(1),
            Some(1),
        )
        .expect("capture")
        .expect("mixed");

        assert!(matches!(capture, CapturedClipboard::Mixed { .. }));
    }

    #[test]
    fn classifies_html_with_img_but_without_png_as_mixed() {
        let settings = AppSettings::default();

        let capture = build_captured_clipboard(
            &settings,
            String::new(),
            Some("<p>hello</p><img src=\"data:image/png;base64,YWJj\" />".into()),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("capture")
        .expect("mixed");

        assert!(matches!(
            capture,
            CapturedClipboard::Mixed {
                png_bytes: None,
                ..
            }
        ));
    }

    #[test]
    fn keeps_plain_html_without_image_as_text() {
        let settings = AppSettings::default();

        let capture = build_captured_clipboard(
            &settings,
            String::new(),
            Some("<p>hello</p><strong>world</strong>".into()),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("capture")
        .expect("text");

        assert!(matches!(capture, CapturedClipboard::Text { .. }));
    }

    #[test]
    fn dto_marks_html_image_src_for_mixed_items_without_png() {
        let item = StoredClipboardItem {
            id: "1".into(),
            kind: "mixed".into(),
            created_at: "2026-04-13T00:00:00Z".into(),
            pinned_at: None,
            preview: "hello".into(),
            full_text: Some("hello".into()),
            html_text: Some("<p>hello</p><img src=\"data:image/png;base64,YWJj\" />".into()),
            rtf_text: None,
            image_png: None,
            image_original_bytes: None,
            image_original_mime: None,
            image_preview_png: None,
            image_width: None,
            image_height: None,
            source_app: None,
            source_icon_data_url: None,
            hash: "hash".into(),
            pinned: false,
            favorite: false,
            tag_colors: Vec::new(),
            copy_count: 0,
            paste_count: 0,
            updated_at: "2026-04-13T00:00:00Z".into(),
            sync_updated_at: "2026-04-13T00:00:00Z".into(),
            sync_device_id: "test-device".into(),
        };

        let dto = history_item_to_dto(&item);

        assert!(dto.image_data_url.is_none());
        assert!(dto.has_image_preview);
        assert_eq!(
            dto.image_preview_url.as_deref(),
            Some("history-preview://localhost/1")
        );
    }

    #[test]
    fn dto_marks_local_html_image_file_for_preview() {
        let root =
            std::env::temp_dir().join(format!("clipdesk-history-test-{}", uuid::Uuid::new_v4()));
        std_fs::create_dir_all(&root).expect("create temp dir");
        let image_path = root.join("preview.png");
        let mut png_bytes = Vec::new();
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255])))
            .write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)
            .expect("png");
        std_fs::write(&image_path, &png_bytes).expect("write png");

        let item = StoredClipboardItem {
            id: "2".into(),
            kind: "mixed".into(),
            created_at: "2026-04-13T00:00:00Z".into(),
            pinned_at: None,
            preview: "hello".into(),
            full_text: Some("hello".into()),
            html_text: Some(format!(
                "<p>hello</p><img src=\"{}\" />",
                image_path.display()
            )),
            rtf_text: None,
            image_png: None,
            image_original_bytes: None,
            image_original_mime: None,
            image_preview_png: None,
            image_width: None,
            image_height: None,
            source_app: None,
            source_icon_data_url: None,
            hash: "hash".into(),
            pinned: false,
            favorite: false,
            tag_colors: Vec::new(),
            copy_count: 0,
            paste_count: 0,
            updated_at: "2026-04-13T00:00:00Z".into(),
            sync_updated_at: "2026-04-13T00:00:00Z".into(),
            sync_device_id: "test-device".into(),
        };

        let dto = history_item_to_dto(&item);

        assert!(dto.image_data_url.is_none());
        assert!(dto.has_image_preview);
        assert_eq!(
            dto.image_preview_url.as_deref(),
            Some("history-preview://localhost/2")
        );

        let _ = std_fs::remove_file(image_path);
        let _ = std_fs::remove_dir_all(root);
    }
}
