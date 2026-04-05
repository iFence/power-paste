use crate::models::ClipboardBackend;

use super::payload::ClipboardPayload;

pub(crate) fn preferred_backend_for_payload(payload: &ClipboardPayload) -> ClipboardBackend {
    match payload {
        ClipboardPayload::Mixed { .. } if cfg!(windows) || cfg!(target_os = "macos") => {
            ClipboardBackend::NativeFallback
        }
        _ => ClipboardBackend::Plugin,
    }
}
