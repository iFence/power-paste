use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use tauri::{AppHandle, Emitter, Listener};
use tauri_plugin_clipboard_next::ClipboardNextExt;

use crate::{
    clipboard::capture_clipboard,
    history::{capture_foreground_app, history_item_to_dto, source_app_info, store_capture},
    models::{CapturedClipboard, SharedState, HISTORY_UPDATED_EVENT},
};

const CLIPBOARD_CHANGE_EVENT: &str = "plugin:clipboard-next://clipboard_change";

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

fn process_clipboard_change(app: AppHandle, shared: Arc<SharedState>) {
    if clipboard_suppression_remaining(&shared).is_some() {
        return;
    }

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
            if suppress_active
                && (monitor.suppress_hash.is_none()
                    || monitor.suppress_hash.as_deref() == Some(hash.as_str()))
            {
                monitor.last_seen_hash = Some(hash);
                return;
            }

            monitor.last_seen_hash = Some(hash.clone());
            drop(monitor);

            let mut store = shared.history_store.lock().unwrap();
            let mut history = shared.history.lock().unwrap();
            if store_capture(
                &mut store,
                &mut history,
                capture,
                source_app.and_then(source_app_info),
                &settings,
            )
            .is_ok()
            {
                if let Some(item) = history.iter().find(|item| item.hash == hash) {
                    let _ = app.emit(HISTORY_UPDATED_EVENT, history_item_to_dto(item));
                }
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
        thread::spawn(move || process_clipboard_change(worker_app, worker_shared));
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
    thread::spawn(move || loop {
        if let Some(remaining) = clipboard_suppression_remaining(&shared) {
            thread::sleep(remaining.min(Duration::from_millis(250)));
            continue;
        }
        process_clipboard_change(app.clone(), shared.clone());
        let settings = shared.settings.lock().unwrap().clone();
        thread::sleep(Duration::from_millis(settings.polling_interval_ms));
    });
}

// Plugin watch is primary; polling remains as a fallback if the watcher fails to start.
pub(crate) fn start_clipboard_monitor(app: AppHandle, shared: Arc<SharedState>) {
    if !start_plugin_watch(&app, shared.clone()) {
        start_fallback_polling(app, shared);
    }
}
