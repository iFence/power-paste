use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use tauri::{AppHandle, Emitter, Listener};
use tauri_plugin_clipboard_next::ClipboardNextExt;

use crate::{
    clipboard::capture_clipboard,
    history::{
        capture_foreground_app, history_item_to_dto, source_app_icon_data_url, source_app_info,
        store_capture_item,
    },
    models::{
        AppSettings, CapturedClipboard, SharedState, COPY_SOUND_EVENT, HISTORY_UPDATED_EVENT,
    },
};

const CLIPBOARD_CHANGE_EVENT: &str = "plugin:clipboard-next://clipboard_change";
const COPY_SOUND_DEBOUNCE: Duration = Duration::from_millis(650);

#[cfg(windows)]
mod windows_clipboard_watch {
    use std::sync::{mpsc, Arc};

    use super::process_clipboard_change;
    use crate::models::SharedState;
    use tauri::AppHandle;
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::DataExchange::{AddClipboardFormatListener, RemoveClipboardFormatListener},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowLongPtrW,
            RegisterClassW, SetWindowLongPtrW, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, HWND_MESSAGE,
            MSG, WM_CLIPBOARDUPDATE, WNDCLASSW,
        },
    };

    struct WatchContext {
        app: AppHandle,
        shared: Arc<SharedState>,
    }

    pub(super) fn start(app: AppHandle, shared: Arc<SharedState>) -> bool {
        let (ready_tx, ready_rx) = mpsc::channel();
        std::thread::spawn(move || unsafe {
            let class_name: Vec<u16> = "PowerPasteClipboardListener\0".encode_utf16().collect();
            let wnd_class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                lpszClassName: class_name.as_ptr(),
                ..Default::default()
            };

            if RegisterClassW(&wnd_class) == 0 {
                let _ = ready_tx.send(false);
                return;
            }

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                std::ptr::null(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            if hwnd.is_null() {
                let _ = ready_tx.send(false);
                return;
            }

            let context = Box::new(WatchContext { app, shared });
            let context_ptr = Box::into_raw(context);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, context_ptr as isize);

            if AddClipboardFormatListener(hwnd) == 0 {
                let _ = Box::from_raw(context_ptr);
                let _ = ready_tx.send(false);
                return;
            }

            let _ = ready_tx.send(true);
            let mut message = MSG::default();
            while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                DispatchMessageW(&message);
            }

            RemoveClipboardFormatListener(hwnd);
            let _ = Box::from_raw(context_ptr);
        });

        ready_rx.recv().unwrap_or(false)
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if message == WM_CLIPBOARDUPDATE {
            let context_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WatchContext;
            if !context_ptr.is_null() {
                let context = &*context_ptr;
                let app = context.app.clone();
                let shared = context.shared.clone();
                std::thread::spawn(move || process_clipboard_change(app, shared, true));
            }
            return 0;
        }

        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}

// Temporarily ignores clipboard changes produced by our own copy/paste actions.
pub(crate) fn mark_clipboard_suppressed(state: &Arc<SharedState>, hash: String) {
    let mut monitor = state.monitor.lock().unwrap();
    monitor.suppress_hash = Some(hash);
    monitor.suppress_until = Some(Instant::now() + Duration::from_secs(2));
}

pub(crate) fn clipboard_suppression_remaining(state: &Arc<SharedState>) -> Option<Duration> {
    let monitor = state.monitor.lock().unwrap();
    monitor
        .suppress_until
        .and_then(|until| until.checked_duration_since(Instant::now()))
}

// 清空历史后重置剪贴板观察状态，允许相同内容再次被捕获。
pub(crate) fn reset_clipboard_observation(state: &Arc<SharedState>) {
    let mut monitor = state.monitor.lock().unwrap();
    monitor.last_seen_hash = None;
    monitor.suppress_hash = None;
    monitor.suppress_until = None;
    monitor.last_sound_event_at = None;
}

// 剪贴板监听确认是外部复制后立即通知前端播放音效，避免等待历史写入造成延迟。
fn emit_captured_copy_sound(app: &AppHandle, settings: &AppSettings) {
    if !settings.sound_enabled {
        return;
    }

    let _ = app.emit(COPY_SOUND_EVENT, ());
}

fn hydrate_source_icon_async(
    app: AppHandle,
    shared: Arc<SharedState>,
    source_app: Option<crate::models::ForegroundAppResult>,
    item_id: String,
) {
    let Some(source_app) = source_app else {
        return;
    };
    if source_app
        .app_path
        .as_deref()
        .unwrap_or_default()
        .is_empty()
        && source_app
            .icon_png_base64
            .as_deref()
            .unwrap_or_default()
            .is_empty()
    {
        return;
    }

    thread::spawn(move || {
        let Some(icon_data_url) = source_app_icon_data_url(&source_app) else {
            return;
        };

        let updated_item = {
            let store = shared.history_store.lock().unwrap();
            if let Err(error) = store.update_source_icon(&item_id, &icon_data_url) {
                eprintln!("source icon update error: {error}");
                return;
            }

            let item = match store.get_item(&item_id) {
                Ok(Some(item)) => item,
                Ok(None) => return,
                Err(error) => {
                    eprintln!("source icon reload error: {error}");
                    return;
                }
            };
            if item.source_icon_data_url.as_deref() != Some(icon_data_url.as_str()) {
                return;
            };
            history_item_to_dto(&item)
        };

        let _ = app.emit(HISTORY_UPDATED_EVENT, updated_item);
    });
}

fn process_clipboard_change(app: AppHandle, shared: Arc<SharedState>, allow_initial_sound: bool) {
    if clipboard_suppression_remaining(&shared).is_some() {
        return;
    }

    let event_started_at = Instant::now();
    let allow_sound_for_event = allow_initial_sound && {
        let mut monitor = shared.monitor.lock().unwrap();
        let duplicate_sound_window = monitor
            .last_sound_event_at
            .and_then(|last| event_started_at.checked_duration_since(last))
            .map(|elapsed| elapsed < COPY_SOUND_DEBOUNCE)
            .unwrap_or(false);
        if duplicate_sound_window {
            false
        } else {
            monitor.last_sound_event_at = Some(event_started_at);
            true
        }
    };

    let settings = shared.settings.lock().unwrap().clone();
    let source_app = match capture_foreground_app() {
        Ok(app) => app,
        Err(error) => {
            eprintln!("foreground app capture error: {error}");
            None
        }
    };

    match capture_clipboard(&app, &settings, source_app.as_ref()) {
        Ok(Some(capture)) => {
            let hash = match &capture {
                CapturedClipboard::Text { hash, .. }
                | CapturedClipboard::Link { hash, .. }
                | CapturedClipboard::Image { hash, .. }
                | CapturedClipboard::Mixed { hash, .. } => hash.clone(),
            };

            let mut monitor = shared.monitor.lock().unwrap();
            if monitor.last_seen_hash.as_deref() == Some(hash.as_str()) {
                return;
            }

            let suppress_active = monitor
                .suppress_until
                .map(|until| until > Instant::now())
                .unwrap_or(false);
            let suppress_hash_matches = monitor.suppress_hash.as_deref() == Some(hash.as_str());
            if (suppress_active && (monitor.suppress_hash.is_none() || suppress_hash_matches))
                || suppress_hash_matches
            {
                monitor.last_seen_hash = Some(hash);
                monitor.suppress_hash = None;
                monitor.suppress_until = None;
                return;
            }

            monitor.last_seen_hash = Some(hash.clone());
            drop(monitor);

            if allow_sound_for_event {
                emit_captured_copy_sound(&app, &settings);
            }

            let source_app_for_icon = source_app.clone();
            let history_item = {
                let mut store = shared.history_store.lock().unwrap();
                match store_capture_item(
                    &mut store,
                    capture,
                    source_app.and_then(source_app_info),
                    &settings,
                ) {
                    Ok(item) => Some(history_item_to_dto(&item)),
                    Err(error) => {
                        eprintln!("clipboard history store error: {error}");
                        None
                    }
                }
            };

            if let Some(item) = history_item {
                hydrate_source_icon_async(
                    app.clone(),
                    shared.clone(),
                    source_app_for_icon,
                    item.id.clone(),
                );
                let _ = app.emit(HISTORY_UPDATED_EVENT, item);
            }
        }
        Ok(None) => {}
        Err(error) => {
            eprintln!("clipboard monitor error: {error}");
        }
    }
}

fn start_plugin_watch(app: &AppHandle, shared: Arc<SharedState>) -> bool {
    let event_app = app.clone();
    app.listen(CLIPBOARD_CHANGE_EVENT, move |_| {
        let worker_app = event_app.clone();
        let worker_shared = shared.clone();
        thread::spawn(move || process_clipboard_change(worker_app, worker_shared, true));
    });

    match app.clipboard_next().start_watch(app.clone()) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("clipboard watch start failed: {error}");
            false
        }
    }
}

fn start_fallback_polling(app: AppHandle, shared: Arc<SharedState>) {
    thread::spawn(move || {
        let mut allow_sound = false;
        loop {
            if let Some(remaining) = clipboard_suppression_remaining(&shared) {
                thread::sleep(remaining.min(Duration::from_millis(250)));
                continue;
            }
            process_clipboard_change(app.clone(), shared.clone(), allow_sound);
            allow_sound = true;
            let settings = shared.settings.lock().unwrap().clone();
            thread::sleep(Duration::from_millis(settings.polling_interval_ms));
        }
    });
}

// Plugin watch is primary; polling remains as a fallback if the watcher fails to start.
pub(crate) fn start_clipboard_monitor(app: AppHandle, shared: Arc<SharedState>) {
    #[cfg(windows)]
    {
        if windows_clipboard_watch::start(app.clone(), shared.clone()) {
            return;
        }
    }

    if !start_plugin_watch(&app, shared.clone()) {
        start_fallback_polling(app, shared);
    }
}
