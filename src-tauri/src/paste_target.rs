use std::{sync::Arc, thread, time::Duration};

use anyhow::Result;
use tauri::AppHandle;
#[cfg(any(windows, target_os = "macos"))]
use tauri::Manager;

#[cfg(target_os = "linux")]
use crate::clipboard::{linux_session_backend, linux_x11_tooling_available};
use crate::models::{SharedState, StoredClipboardItem};

#[cfg(target_os = "macos")]
use crate::clipboard::wait_for_clipboard_payload;
#[cfg(windows)]
use crate::clipboard::{write_image_to_clipboard, write_image_with_plugin, write_text_with_plugin};
#[cfg(windows)]
use crate::models::{HwndRaw, PANEL_LABEL};
#[cfg(target_os = "macos")]
use core::ffi::c_void;
#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};
#[cfg(target_os = "macos")]
use objc2_foundation::NSString;
#[cfg(target_os = "macos")]
use std::process::Command;

#[cfg(windows)]
use std::mem;
#[cfg(windows)]
use std::path::PathBuf;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetProfile {
    Generic,
    #[cfg(windows)]
    Office,
    #[cfg(windows)]
    Wps,
    #[cfg(windows)]
    Markdown,
    #[cfg(windows)]
    Chat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedPasteTarget {
    pub(crate) profile: TargetProfile,
}

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
fn target_profile_for_process_name(process_name: Option<&str>) -> TargetProfile {
    let Some(process_name) = process_name else {
        return TargetProfile::Generic;
    };

    if process_name == "wps" {
        TargetProfile::Wps
    } else if process_name.contains("winword") {
        TargetProfile::Office
    } else if process_name.contains("obsidian") || process_name.contains("typora") {
        TargetProfile::Markdown
    } else if process_name.contains("dingtalk") {
        TargetProfile::Chat
    } else {
        TargetProfile::Generic
    }
}

#[cfg(windows)]
fn last_target_profile(state: &Arc<SharedState>) -> TargetProfile {
    let target = {
        let monitor = state.monitor.lock().unwrap();
        monitor.last_target_window
    };

    let process_name = target.and_then(process_name_for_window);
    target_profile_for_process_name(process_name.as_deref())
}

#[cfg(not(windows))]
fn last_target_profile(_state: &Arc<SharedState>) -> TargetProfile {
    TargetProfile::Generic
}

pub(crate) fn resolve_last_target(state: &Arc<SharedState>) -> ResolvedPasteTarget {
    ResolvedPasteTarget {
        profile: last_target_profile(state),
    }
}

#[cfg(target_os = "macos")]
fn current_foreground_app_info() -> Option<(String, String)> {
    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication()?;
    let bundle_id = app.bundleIdentifier()?.to_string();
    let app_name = app.localizedName()?.to_string();
    if bundle_id.is_empty() || app_name.is_empty() {
        None
    } else {
        Some((bundle_id, app_name))
    }
}

#[cfg(target_os = "macos")]
fn running_application_for_bundle_id(bundle_id: &str) -> Option<Retained<NSRunningApplication>> {
    let bundle_id = NSString::from_str(bundle_id);
    NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle_id).firstObject()
}

#[cfg(target_os = "macos")]
fn normalize_macos_automation_error(message: &str) -> &'static str {
    let lower = message.to_lowercase();
    if lower.contains("not authorized")
        || lower.contains("not permitted")
        || lower.contains("not allowed")
        || lower.contains("assistive access")
        || lower.contains("accessibility")
        || lower.contains("automation")
        || lower.contains("system events got an error")
        || lower.contains("(-1743)")
        || lower.contains("(-1719)")
        || lower.contains("(-25211)")
    {
        "paste_target_permission_denied"
    } else {
        "paste_target_focus_failed"
    }
}

#[cfg(target_os = "macos")]
fn run_macos_osascript(args: &[String]) -> Result<()> {
    let output = Command::new("osascript").args(args).output()?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        anyhow::bail!("paste_target_focus_failed");
    }

    anyhow::bail!(normalize_macos_automation_error(&stderr))
}

#[cfg(target_os = "macos")]
fn ensure_macos_direct_paste_permissions(state: &Arc<SharedState>) -> Result<()> {
    if state
        .macos_direct_paste_permission_verified
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Ok(());
    }

    run_macos_osascript(&[
        "-e".to_string(),
        r#"tell application "System Events" to count of application processes"#.to_string(),
    ])?;
    state
        .macos_direct_paste_permission_verified
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

#[cfg(windows)]
fn current_foreground_window() -> Option<HwndRaw> {
    let hwnd = unsafe { GetForegroundWindow() };
    (hwnd != 0).then_some(hwnd)
}

#[cfg(windows)]
fn paste_debug_enabled(state: &Arc<SharedState>) -> bool {
    state.settings.lock().unwrap().debug_enabled
}

#[cfg(target_os = "macos")]
fn paste_debug_enabled(state: &Arc<SharedState>) -> bool {
    state.settings.lock().unwrap().debug_enabled
}

#[cfg(windows)]
fn debug_log_target_event(
    state: &Arc<SharedState>,
    stage: &str,
    target: Option<HwndRaw>,
    foreground: Option<HwndRaw>,
    reason: &str,
) {
    if !paste_debug_enabled(state) {
        return;
    }

    let target_process = target.and_then(process_name_for_window);
    let foreground_process = foreground.and_then(process_name_for_window);
    eprintln!(
        "[paste-target] stage={stage} target={target:?} target_process={target_process:?} foreground={foreground:?} foreground_process={foreground_process:?} reason={reason}"
    );
}

#[cfg(target_os = "macos")]
fn debug_log_macos_target_event(
    state: &Arc<SharedState>,
    stage: &str,
    target_bundle_id: Option<&str>,
    foreground_bundle_id: Option<&str>,
    reason: &str,
) {
    if !paste_debug_enabled(state) {
        return;
    }

    eprintln!(
        "[paste-target] stage={stage} target_bundle_id={target_bundle_id:?} foreground_bundle_id={foreground_bundle_id:?} reason={reason}"
    );
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
        debug_log_target_event(shared.inner(), "remember-skip", None, None, "no-foreground");
        return;
    };
    if foreground == panel_hwnd {
        debug_log_target_event(
            shared.inner(),
            "remember-skip",
            Some(panel_hwnd),
            Some(foreground),
            "panel-foreground",
        );
        return;
    }

    let mut monitor = shared.inner().monitor.lock().unwrap();
    monitor.last_target_window = Some(foreground);
    debug_log_target_event(
        shared.inner(),
        "remember-target",
        Some(foreground),
        Some(foreground),
        "target-updated",
    );
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub(crate) fn remember_last_target_window(_app: &AppHandle) {}

#[cfg(target_os = "macos")]
pub(crate) fn remember_last_target_window(app: &AppHandle) {
    let Some(shared) = app.try_state::<Arc<SharedState>>() else {
        return;
    };
    let Some((bundle_id, app_name)) = current_foreground_app_info() else {
        return;
    };

    if bundle_id == "com.yulei.powerpaste" {
        debug_log_macos_target_event(
            shared.inner(),
            "remember-skip",
            Some("com.yulei.powerpaste"),
            Some("com.yulei.powerpaste"),
            "panel-foreground",
        );
        return;
    }

    let mut monitor = shared.inner().monitor.lock().unwrap();
    monitor.last_target_app_bundle_id = Some(bundle_id);
    monitor.last_target_app_name = Some(app_name);
    let target_bundle_id = monitor.last_target_app_bundle_id.clone();
    debug_log_macos_target_event(
        shared.inner(),
        "remember-target",
        target_bundle_id.as_deref(),
        target_bundle_id.as_deref(),
        "target-updated",
    );
}

#[cfg(target_os = "linux")]
fn current_foreground_window_id() -> Option<String> {
    if linux_session_backend() != "x11" || !linux_x11_tooling_available() {
        return None;
    }

    let output = std::process::Command::new("xdotool")
        .args(["getactivewindow"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let window_id = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!window_id.is_empty()).then_some(window_id)
}

#[cfg(target_os = "linux")]
pub(crate) fn remember_last_target_window(app: &AppHandle) {
    let Some(shared) = app.try_state::<Arc<SharedState>>() else {
        return;
    };
    let Some(window_id) = current_foreground_window_id() else {
        return;
    };

    let mut monitor = shared.inner().monitor.lock().unwrap();
    monitor.last_target_window_id = Some(window_id);
}

#[cfg(windows)]
// Focus is retried because SetForegroundWindow is not always immediate on Windows.
pub(crate) fn focus_last_target_window(state: &Arc<SharedState>) -> Result<()> {
    const SW_RESTORE: i32 = 9;
    const FOCUS_RETRY_COUNT: usize = 8;
    const FOCUS_RETRY_DELAY_MS: u64 = 5;

    let target = {
        let monitor = state.monitor.lock().unwrap();
        monitor.last_target_window
    };

    let Some(hwnd) = target else {
        debug_log_target_event(
            state,
            "focus-failed",
            None,
            current_foreground_window(),
            "missing-target",
        );
        anyhow::bail!("paste_target_focus_failed");
    };
    if unsafe { IsWindow(hwnd) == 0 } {
        debug_log_target_event(
            state,
            "focus-failed",
            Some(hwnd),
            current_foreground_window(),
            "invalid-target",
        );
        anyhow::bail!("paste_target_focus_failed");
    }

    unsafe {
        if IsIconic(hwnd as HWND) != 0 {
            ShowWindow(hwnd as HWND, SW_RESTORE);
        }
        SetForegroundWindow(hwnd as HWND);
    }
    for _ in 0..FOCUS_RETRY_COUNT {
        if current_foreground_window() == Some(hwnd) {
            debug_log_target_event(
                state,
                "focus-success",
                Some(hwnd),
                Some(hwnd),
                "foreground-matched",
            );
            return Ok(());
        }
        thread::sleep(Duration::from_millis(FOCUS_RETRY_DELAY_MS));
    }

    let foreground = current_foreground_window();
    debug_log_target_event(
        state,
        "focus-failed",
        Some(hwnd),
        foreground,
        "foreground-mismatch",
    );
    anyhow::bail!("paste_target_focus_failed")
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub(crate) fn focus_last_target_window(_state: &Arc<SharedState>) -> Result<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn focus_last_target_window(state: &Arc<SharedState>) -> Result<()> {
    let bundle_id = {
        let monitor = state.monitor.lock().unwrap();
        monitor.last_target_app_bundle_id.clone()
    };

    let Some(bundle_id) = bundle_id.filter(|value| !value.is_empty()) else {
        debug_log_macos_target_event(state, "focus-failed", None, None, "missing-target");
        anyhow::bail!("paste_target_focus_failed");
    };

    let Some(app) = running_application_for_bundle_id(&bundle_id) else {
        debug_log_macos_target_event(
            state,
            "focus-failed",
            Some(bundle_id.as_str()),
            current_foreground_app_info().map(|(id, _)| id).as_deref(),
            "missing-target-process",
        );
        anyhow::bail!("paste_target_focus_failed");
    };

    if app.isHidden() {
        let _ = app.unhide();
    }

    #[allow(deprecated)]
    let options = NSApplicationActivationOptions::ActivateAllWindows
        | NSApplicationActivationOptions::ActivateIgnoringOtherApps;
    if !app.activateWithOptions(options) {
        anyhow::bail!("paste_target_focus_failed");
    }

    debug_log_macos_target_event(
        state,
        "focus-requested",
        Some(bundle_id.as_str()),
        current_foreground_app_info().map(|(id, _)| id).as_deref(),
        "activation-dispatched",
    );
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_linux_xdotool(args: &[&str]) -> Result<()> {
    if linux_session_backend() == "wayland" {
        anyhow::bail!("linux_wayland_unsupported");
    }
    if !linux_x11_tooling_available() {
        anyhow::bail!("linux_x11_tools_missing");
    }

    let output = std::process::Command::new("xdotool").args(args).output()?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        anyhow::bail!("paste_target_focus_failed");
    }

    anyhow::bail!(stderr)
}

#[cfg(target_os = "linux")]
pub(crate) fn focus_last_target_window(state: &Arc<SharedState>) -> Result<()> {
    let window_id = {
        let monitor = state.monitor.lock().unwrap();
        monitor.last_target_window_id.clone()
    };

    let Some(window_id) = window_id.filter(|value| !value.is_empty()) else {
        anyhow::bail!("paste_target_focus_failed");
    };

    run_linux_xdotool(&["windowactivate", "--sync", window_id.as_str()])
}

#[cfg(target_os = "macos")]
pub(crate) fn wait_for_paste_target_focus(state: &Arc<SharedState>) {
    const FOCUS_RETRY_COUNT: usize = 10;
    const FOCUS_RETRY_DELAY_MS: u64 = 20;

    let target_bundle_id = {
        let monitor = state.monitor.lock().unwrap();
        monitor.last_target_app_bundle_id.clone()
    };

    let Some(target_bundle_id) = target_bundle_id else {
        thread::sleep(Duration::from_millis(120));
        return;
    };

    for _ in 0..FOCUS_RETRY_COUNT {
        if current_foreground_app_info()
            .map(|(bundle_id, _)| bundle_id == target_bundle_id)
            .unwrap_or(false)
        {
            debug_log_macos_target_event(
                state,
                "focus-success",
                Some(target_bundle_id.as_str()),
                Some(target_bundle_id.as_str()),
                "foreground-matched",
            );
            return;
        }
        thread::sleep(Duration::from_millis(FOCUS_RETRY_DELAY_MS));
    }

    let foreground_bundle_id = current_foreground_app_info().map(|(bundle_id, _)| bundle_id);
    debug_log_macos_target_event(
        state,
        "focus-failed",
        Some(target_bundle_id.as_str()),
        foreground_bundle_id.as_deref(),
        "foreground-mismatch",
    );
}

#[cfg(windows)]
pub(crate) fn wait_for_paste_target_focus(_state: &Arc<SharedState>) {
    thread::sleep(Duration::from_millis(180));
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub(crate) fn wait_for_paste_target_focus(_state: &Arc<SharedState>) {}

#[cfg(target_os = "linux")]
pub(crate) fn wait_for_paste_target_focus(state: &Arc<SharedState>) {
    const FOCUS_RETRY_COUNT: usize = 10;
    const FOCUS_RETRY_DELAY_MS: u64 = 20;

    let target_window_id = {
        let monitor = state.monitor.lock().unwrap();
        monitor.last_target_window_id.clone()
    };

    let Some(target_window_id) = target_window_id else {
        thread::sleep(Duration::from_millis(120));
        return;
    };

    for _ in 0..FOCUS_RETRY_COUNT {
        if current_foreground_window_id()
            .map(|window_id| window_id == target_window_id)
            .unwrap_or(false)
        {
            return;
        }
        thread::sleep(Duration::from_millis(FOCUS_RETRY_DELAY_MS));
    }
}

#[cfg(windows)]
pub(crate) fn send_native_paste_shortcut(_state: &Arc<SharedState>) -> Result<()> {
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

    Ok(())
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub(crate) fn send_native_paste_shortcut(_state: &Arc<SharedState>) -> Result<()> {
    Ok(())
}

#[cfg(target_os = "linux")]
pub(crate) fn send_native_paste_shortcut(state: &Arc<SharedState>) -> Result<()> {
    let window_id = {
        let monitor = state.monitor.lock().unwrap();
        monitor.last_target_window_id.clone()
    };

    let Some(window_id) = window_id.filter(|value| !value.is_empty()) else {
        anyhow::bail!("paste_target_focus_failed");
    };

    if current_foreground_window_id().as_deref() != Some(window_id.as_str()) {
        anyhow::bail!("paste_target_focus_failed");
    }

    run_linux_xdotool(&[
        "key",
        "--window",
        window_id.as_str(),
        "--clearmodifiers",
        "ctrl+v",
    ])
}

#[cfg(target_os = "macos")]
type CGEventRef = *mut c_void;
#[cfg(target_os = "macos")]
type CGEventFlags = u64;
#[cfg(target_os = "macos")]
type CGEventTapLocation = u32;
#[cfg(target_os = "macos")]
type CGKeyCode = u16;

#[cfg(target_os = "macos")]
const KCG_SESSION_EVENT_TAP: CGEventTapLocation = 1;
#[cfg(target_os = "macos")]
const KCG_EVENT_FLAG_MASK_COMMAND: CGEventFlags = 0x0010_0000;
#[cfg(target_os = "macos")]
const MACOS_KEYCODE_COMMAND: CGKeyCode = 0x37;
#[cfg(target_os = "macos")]
const MACOS_KEYCODE_V: CGKeyCode = 0x09;

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreateKeyboardEvent(
        source: *const c_void,
        virtual_key: CGKeyCode,
        key_down: bool,
    ) -> CGEventRef;
    fn CGEventSetFlags(event: CGEventRef, flags: CGEventFlags);
    fn CGEventPost(tap: CGEventTapLocation, event: CGEventRef);
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: *const c_void);
}

#[cfg(target_os = "macos")]
fn post_macos_keyboard_event(
    keycode: CGKeyCode,
    key_down: bool,
    flags: CGEventFlags,
) -> Result<()> {
    let event = unsafe { CGEventCreateKeyboardEvent(std::ptr::null(), keycode, key_down) };
    if event.is_null() {
        anyhow::bail!("paste_target_focus_failed");
    }

    unsafe {
        CGEventSetFlags(event, flags);
        CGEventPost(KCG_SESSION_EVENT_TAP, event);
        CFRelease(event.cast());
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn send_native_paste_shortcut(state: &Arc<SharedState>) -> Result<()> {
    let bundle_id = {
        let monitor = state.monitor.lock().unwrap();
        monitor.last_target_app_bundle_id.clone()
    };
    let app_name = {
        let monitor = state.monitor.lock().unwrap();
        monitor.last_target_app_name.clone()
    };

    if current_foreground_app_info()
        .map(|(foreground_bundle_id, _)| Some(foreground_bundle_id) == bundle_id)
        != Some(true)
    {
        anyhow::bail!("paste_target_focus_failed");
    }

    if app_name.filter(|value| !value.is_empty()).is_none() {
        anyhow::bail!("paste_target_focus_failed");
    }

    post_macos_keyboard_event(MACOS_KEYCODE_COMMAND, true, KCG_EVENT_FLAG_MASK_COMMAND)?;
    post_macos_keyboard_event(MACOS_KEYCODE_V, true, KCG_EVENT_FLAG_MASK_COMMAND)?;
    post_macos_keyboard_event(MACOS_KEYCODE_V, false, KCG_EVENT_FLAG_MASK_COMMAND)?;
    post_macos_keyboard_event(MACOS_KEYCODE_COMMAND, false, 0)?;
    Ok(())
}

pub(crate) fn prepare_target_for_paste(state: &Arc<SharedState>) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        ensure_macos_direct_paste_permissions(state)?;
    }
    focus_last_target_window(state)?;
    wait_for_paste_target_focus(state);
    #[cfg(windows)]
    {
        let target = {
            let monitor = state.monitor.lock().unwrap();
            monitor.last_target_window
        };
        let foreground = current_foreground_window();
        if target.is_none() || foreground != target {
            debug_log_target_event(
                state,
                "prepare-abort",
                target,
                foreground,
                "focus-verification-failed",
            );
            anyhow::bail!("paste_target_focus_failed");
        }
    }
    #[cfg(target_os = "macos")]
    {
        let target_bundle_id = {
            let monitor = state.monitor.lock().unwrap();
            monitor.last_target_app_bundle_id.clone()
        };
        let focused_bundle_id = current_foreground_app_info().map(|(bundle_id, _)| bundle_id);
        if target_bundle_id.is_none() || focused_bundle_id != target_bundle_id {
            anyhow::bail!("paste_target_focus_failed");
        }
    }
    #[cfg(target_os = "linux")]
    {
        let target_window_id = {
            let monitor = state.monitor.lock().unwrap();
            monitor.last_target_window_id.clone()
        };
        let focused_window_id = current_foreground_window_id();
        if target_window_id.is_none() || focused_window_id != target_window_id {
            anyhow::bail!("paste_target_focus_failed");
        }
    }
    Ok(())
}

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
    item.image_png
        .as_ref()
        .map(|bytes| !bytes.is_empty())
        .unwrap_or(false)
}

#[cfg(windows)]
// Markdown/chat targets often need text and image pasted as multiple sequential operations.
fn paste_mixed_segments(
    app: &AppHandle,
    state: &Arc<SharedState>,
    segments: &[crate::clipboard_html::MixedPasteSegment],
    png_bytes: &[u8],
) -> Result<bool> {
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
                write_text_with_plugin(app, text)?;
            }
            crate::clipboard_html::MixedPasteSegment::Image => {
                write_image_with_plugin(app, png_bytes)
                    .or_else(|_| write_image_to_clipboard(png_bytes))?;
            }
            crate::clipboard_html::MixedPasteSegment::Text(_) => continue,
        }
        thread::sleep(Duration::from_millis(120));
        send_native_paste_shortcut(state)?;
        thread::sleep(Duration::from_millis(120));
    }

    Ok(true)
}

#[cfg(windows)]
// Only a subset of targets need segmented mixed paste; everything else uses the normal clipboard write path.
pub(crate) fn paste_mixed_item_for_profile(
    app: &AppHandle,
    state: &Arc<SharedState>,
    item: &StoredClipboardItem,
    profile: TargetProfile,
) -> Result<bool> {
    if item.kind != "mixed" || !item_has_image(item) {
        return Ok(false);
    }

    match profile {
        TargetProfile::Markdown | TargetProfile::Chat => {
            let png_bytes = item
                .image_png
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
                        segments.push(crate::clipboard_html::MixedPasteSegment::Text(
                            text.to_string(),
                        ));
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

            paste_mixed_segments(app, state, &segments, png_bytes)
        }
        _ => Ok(false),
    }
}

#[cfg(not(windows))]
pub(crate) fn paste_mixed_item_for_profile(
    _app: &AppHandle,
    _state: &Arc<SharedState>,
    _item: &StoredClipboardItem,
    _profile: TargetProfile,
) -> Result<bool> {
    Ok(false)
}

pub(crate) fn paste_item_to_target(
    app: &AppHandle,
    state: &Arc<SharedState>,
    item: &StoredClipboardItem,
    target: &ResolvedPasteTarget,
) -> Result<bool> {
    if paste_mixed_item_for_profile(app, state, item, target.profile)? {
        return Ok(true);
    }

    #[cfg(target_os = "macos")]
    let written_payload =
        crate::clipboard::write_item_to_clipboard_with_profile(app, item, target.profile)?;
    #[cfg(not(target_os = "macos"))]
    crate::clipboard::write_item_to_clipboard_with_profile(app, item, target.profile)?;
    #[cfg(target_os = "macos")]
    {
        wait_for_clipboard_payload(app, &written_payload)?;
        wait_for_paste_target_focus(state);
    }
    #[cfg(not(target_os = "macos"))]
    {
        thread::sleep(Duration::from_millis(180));
    }
    send_native_paste_shortcut(state)?;
    Ok(true)
}
