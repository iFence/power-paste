use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use tauri::{AppHandle, Emitter};

use crate::{
    history::{capture_clipboard, capture_foreground_app, source_app_info, store_capture},
    models::{CapturedClipboard, SharedState, HISTORY_UPDATED_EVENT},
};

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

// Polling is deliberate here because clipboard ownership and mixed payload capture are platform-specific.
pub(crate) fn start_clipboard_monitor(app: AppHandle, shared: Arc<SharedState>) {
    thread::spawn(move || loop {
        if let Some(remaining) = clipboard_suppression_remaining(&shared) {
            thread::sleep(remaining.min(Duration::from_millis(250)));
            continue;
        }

        let settings = shared.settings.lock().unwrap().clone();
        let source_app = match capture_foreground_app() {
            Ok(app) => app,
            Err(error) => {
                eprintln!("foreground app capture error: {error}");
                None
            }
        };

        match capture_clipboard(&settings, source_app.as_ref()) {
            Ok(Some(capture)) => {
                let hash = match &capture {
                    CapturedClipboard::Text { hash, .. }
                    | CapturedClipboard::Image { hash, .. }
                    | CapturedClipboard::Mixed { hash, .. } => hash.clone(),
                };

                let mut monitor = shared.monitor.lock().unwrap();
                if monitor.last_seen_hash.as_deref() == Some(hash.as_str()) {
                    drop(monitor);
                    thread::sleep(Duration::from_millis(settings.polling_interval_ms));
                    continue;
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
                    drop(monitor);
                    thread::sleep(Duration::from_millis(settings.polling_interval_ms));
                    continue;
                }

                monitor.last_seen_hash = Some(hash);
                drop(monitor);

                let mut history = shared.history.lock().unwrap();
                if store_capture(
                    &shared.paths,
                    &mut history,
                    capture,
                    source_app.and_then(source_app_info),
                    &settings,
                )
                .unwrap_or(false)
                {
                    let _ = app.emit(HISTORY_UPDATED_EVENT, ());
                }
            }
            Ok(None) => {}
            Err(error) => {
                eprintln!("clipboard monitor error: {error}");
            }
        }

        thread::sleep(Duration::from_millis(settings.polling_interval_ms));
    });
}
