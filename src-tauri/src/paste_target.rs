use std::{path::PathBuf, sync::Arc, thread, time::Duration};

use anyhow::Result;
use tauri::{AppHandle, Manager};

use crate::models::{ClipboardTargetProfile, HwndRaw, SharedState, StoredClipboardItem, PANEL_LABEL};

#[cfg(windows)]
use std::mem;
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{CloseHandle, HWND},
    System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    },
    UI::{
        Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL,
            VK_V,
        },
        WindowsAndMessaging::{
            GetWindowThreadProcessId, IsIconic, SetForegroundWindow, ShowWindow,
        },
    },
};

// Tracks the last non-panel foreground app so paste can return to the right target window.
#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn GetForegroundWindow() -> HwndRaw;
    fn IsWindow(hwnd: HwndRaw) -> i32;
}

#[cfg(windows)]
fn process_name_for_window(hwnd: HwndRaw) -> Option<String> {
    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd as HWND, &mut process_id);
    }
    if process_id == 0 {
        return None;
    }

    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if process.is_null() {
        return None;
    }

    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    let ok = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            buffer.as_mut_ptr(),
            &mut length,
        )
    };
    unsafe {
        CloseHandle(process);
    }
    if ok == 0 || length == 0 {
        return None;
    }

    let path = String::from_utf16_lossy(&buffer[..length as usize]);
    PathBuf::from(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
}

#[cfg(windows)]
// Different targets expect different clipboard payload shapes, especially Office/Markdown/chat apps.
fn target_profile_for_process_name(process_name: Option<&str>) -> ClipboardTargetProfile {
    let Some(process_name) = process_name else {
        return ClipboardTargetProfile::Generic;
    };

    if process_name == "wps" {
        ClipboardTargetProfile::Wps
    } else if process_name.contains("winword") {
        ClipboardTargetProfile::Office
    } else if process_name.contains("obsidian") || process_name.contains("typora") {
        ClipboardTargetProfile::Markdown
    } else if process_name.contains("dingtalk") {
        ClipboardTargetProfile::Chat
    } else {
        ClipboardTargetProfile::Generic
    }
}

#[cfg(windows)]
pub(crate) fn last_target_profile(state: &Arc<SharedState>) -> ClipboardTargetProfile {
    let target = {
        let monitor = state.monitor.lock().unwrap();
        monitor.last_target_window
    };

    let process_name = target.and_then(process_name_for_window);
    target_profile_for_process_name(process_name.as_deref())
}

#[cfg(not(windows))]
pub(crate) fn last_target_profile(_state: &Arc<SharedState>) -> ClipboardTargetProfile {
    ClipboardTargetProfile::Generic
}

#[cfg(windows)]
fn current_foreground_window() -> Option<HwndRaw> {
    let hwnd = unsafe { GetForegroundWindow() };
    (hwnd != 0).then_some(hwnd)
}

#[cfg(windows)]
pub(crate) fn remember_last_target_window(app: &AppHandle) {
    let Some(shared) = app.try_state::<Arc<SharedState>>() else {
        return;
    };
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        return;
    };
    let Ok(panel_hwnd) = window.hwnd() else {
        return;
    };
    let panel_hwnd = panel_hwnd.0 as HwndRaw;
    let Some(foreground) = current_foreground_window() else {
        return;
    };
    if foreground == panel_hwnd {
        return;
    }

    let mut monitor = shared.inner().monitor.lock().unwrap();
    monitor.last_target_window = Some(foreground);
}

#[cfg(not(windows))]
pub(crate) fn remember_last_target_window(_app: &AppHandle) {}

#[cfg(windows)]
// Focus is retried because SetForegroundWindow is not always immediate on Windows.
pub(crate) fn focus_last_target_window(state: &Arc<SharedState>) {
    const SW_RESTORE: i32 = 9;
    const FOCUS_RETRY_COUNT: usize = 8;
    const FOCUS_RETRY_DELAY_MS: u64 = 5;

    let target = {
        let monitor = state.monitor.lock().unwrap();
        monitor.last_target_window
    };

    if let Some(hwnd) = target.filter(|hwnd| unsafe { IsWindow(*hwnd) != 0 }) {
        unsafe {
            if IsIconic(hwnd as HWND) != 0 {
                ShowWindow(hwnd as HWND, SW_RESTORE);
            }
            SetForegroundWindow(hwnd as HWND);
        }
        for _ in 0..FOCUS_RETRY_COUNT {
            if current_foreground_window() == Some(hwnd) {
                break;
            }
            thread::sleep(Duration::from_millis(FOCUS_RETRY_DELAY_MS));
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn focus_last_target_window(_state: &Arc<SharedState>) {}

#[cfg(windows)]
pub(crate) fn wait_for_paste_target_focus() {
    thread::sleep(Duration::from_millis(180));
}

#[cfg(not(windows))]
pub(crate) fn wait_for_paste_target_focus() {}

#[cfg(windows)]
pub(crate) fn send_native_paste_shortcut() {
    let mut inputs = [
        keyboard_input(VK_CONTROL as u16, 0),
        keyboard_input(VK_V as u16, 0),
        keyboard_input(VK_V as u16, KEYEVENTF_KEYUP),
        keyboard_input(VK_CONTROL as u16, KEYEVENTF_KEYUP),
    ];

    unsafe {
        let _ = SendInput(
            inputs.len() as u32,
            inputs.as_mut_ptr(),
            mem::size_of::<INPUT>() as i32,
        );
    }
}

#[cfg(not(windows))]
pub(crate) fn send_native_paste_shortcut() {}

#[cfg(windows)]
fn keyboard_input(virtual_key: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: virtual_key,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(windows)]
fn item_has_image(item: &StoredClipboardItem) -> bool {
    item.image_path
        .as_deref()
        .map(|path| !path.is_empty())
        .unwrap_or(false)
}

#[cfg(windows)]
// Markdown/chat targets often need text and image pasted as multiple sequential operations.
fn paste_mixed_segments(segments: &[crate::clipboard_html::MixedPasteSegment], image_path: &str) -> Result<bool> {
    let has_content = segments.iter().any(|segment| match segment {
        crate::clipboard_html::MixedPasteSegment::Text(text) => !text.is_empty(),
        crate::clipboard_html::MixedPasteSegment::Image => true,
    });
    if !has_content {
        return Ok(false);
    }

    for segment in segments {
        match segment {
            crate::clipboard_html::MixedPasteSegment::Text(text) if !text.is_empty() => {
                crate::clipboard_write::write_unicode_text_to_clipboard(text)?;
            }
            crate::clipboard_html::MixedPasteSegment::Image => {
                crate::clipboard_write::write_image_to_clipboard(image_path)?;
            }
            crate::clipboard_html::MixedPasteSegment::Text(_) => continue,
        }
        thread::sleep(Duration::from_millis(120));
        send_native_paste_shortcut();
        thread::sleep(Duration::from_millis(120));
    }

    Ok(true)
}

#[cfg(not(windows))]
fn paste_mixed_segments(
    _segments: &[crate::clipboard_html::MixedPasteSegment],
    _image_path: &str,
) -> Result<bool> {
    Ok(false)
}

#[cfg(windows)]
// Only a subset of targets need segmented mixed paste; everything else uses the normal clipboard write path.
pub(crate) fn paste_mixed_item_for_profile(
    item: &StoredClipboardItem,
    profile: ClipboardTargetProfile,
) -> Result<bool> {
    if item.kind != "mixed" || !item_has_image(item) {
        return Ok(false);
    }

    match profile {
        ClipboardTargetProfile::Markdown | ClipboardTargetProfile::Chat => {
            let image_path = item
                .image_path
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Image payload missing"))?;
            let segments = item
                .html_text
                .as_deref()
                .map(crate::clipboard_html::html_to_mixed_segments)
                .filter(|segments| !segments.is_empty())
                .unwrap_or_else(|| {
                    let mut segments = Vec::new();
                    if let Some(text) = item.full_text.as_deref().filter(|text| !text.is_empty()) {
                        segments.push(crate::clipboard_html::MixedPasteSegment::Text(text.to_string()));
                    }
                    segments.push(crate::clipboard_html::MixedPasteSegment::Image);
                    segments
                });
            let segments = item
                .full_text
                .as_deref()
                .filter(|text| !text.is_empty())
                .map(|text| crate::clipboard_html::remap_mixed_text_segments(&segments, text))
                .unwrap_or(segments);

            paste_mixed_segments(&segments, image_path)
        }
        _ => Ok(false),
    }
}

#[cfg(not(windows))]
pub(crate) fn paste_mixed_item_for_profile(
    _item: &StoredClipboardItem,
    _profile: ClipboardTargetProfile,
) -> Result<bool> {
    Ok(false)
}
