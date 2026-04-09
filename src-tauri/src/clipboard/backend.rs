use crate::models::ClipboardBackend;

use super::payload::ClipboardPayload;

pub(crate) fn preferred_backend_for_payload(payload: &ClipboardPayload) -> ClipboardBackend {
    match payload {
        ClipboardPayload::Html { .. } if cfg!(windows) => ClipboardBackend::NativeFallback,
        ClipboardPayload::RichText { html, .. } if cfg!(windows) && html.is_some() => {
            ClipboardBackend::NativeFallback
        }
        ClipboardPayload::Mixed { .. } if cfg!(windows) || cfg!(target_os = "macos") => {
            ClipboardBackend::NativeFallback
        }
        _ => ClipboardBackend::Plugin,
    }
}

#[cfg(test)]
mod tests {
    use super::preferred_backend_for_payload;
    use crate::models::ClipboardBackend;
    use crate::clipboard::payload::ClipboardPayload;

    #[test]
    fn prefers_native_fallback_for_html_on_windows() {
        let payload = ClipboardPayload::Html {
            text: Some('修'.to_string()),
            html: "<b>修</b>".into(),
        };

        let backend = preferred_backend_for_payload(&payload);

        if cfg!(windows) {
            assert!(matches!(backend, ClipboardBackend::NativeFallback));
        } else {
            assert!(matches!(backend, ClipboardBackend::Plugin));
        }
    }
}
