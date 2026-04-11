use crate::models::ClipboardBackend;

use super::payload::ClipboardPayload;

pub(crate) fn preferred_backend_for_payload(payload: &ClipboardPayload) -> ClipboardBackend {
    match payload {
        ClipboardPayload::Image { .. } if cfg!(target_os = "macos") => {
            ClipboardBackend::NativeFallback
        }
        ClipboardPayload::Html { .. } if cfg!(target_os = "macos") => {
            ClipboardBackend::NativeFallback
        }
        ClipboardPayload::RichText { html, rtf, .. }
            if cfg!(target_os = "macos") && (html.is_some() || rtf.is_some()) =>
        {
            ClipboardBackend::NativeFallback
        }
        ClipboardPayload::Mixed { .. } if cfg!(target_os = "macos") => {
            ClipboardBackend::NativeFallback
        }
        ClipboardPayload::Html { .. } if cfg!(windows) => ClipboardBackend::NativeFallback,
        ClipboardPayload::RichText { html, .. } if cfg!(windows) && html.is_some() => {
            ClipboardBackend::NativeFallback
        }
        ClipboardPayload::Mixed { .. } if cfg!(windows) => ClipboardBackend::NativeFallback,
        _ => ClipboardBackend::Plugin,
    }
}

#[cfg(test)]
mod tests {
    use super::preferred_backend_for_payload;
    use crate::clipboard::payload::ClipboardPayload;
    use crate::models::ClipboardBackend;

    #[test]
    fn prefers_native_fallback_for_html_payload() {
        let payload = ClipboardPayload::Html {
            text: Some('修'.to_string()),
            html: "<b>修</b>".into(),
        };

        let backend = preferred_backend_for_payload(&payload);

        if cfg!(windows) || cfg!(target_os = "macos") {
            assert!(matches!(backend, ClipboardBackend::NativeFallback));
        } else {
            assert!(matches!(backend, ClipboardBackend::Plugin));
        }
    }

    #[test]
    fn prefers_native_fallback_for_mixed_payload_on_desktop_native_paths() {
        let payload = ClipboardPayload::Mixed {
            text: Some("text".into()),
            html: Some("<b>text</b>".into()),
            png_bytes: vec![1, 2, 3],
        };

        let backend = preferred_backend_for_payload(&payload);

        if cfg!(windows) || cfg!(target_os = "macos") {
            assert!(matches!(backend, ClipboardBackend::NativeFallback));
        } else {
            assert!(matches!(backend, ClipboardBackend::Plugin));
        }
    }
}
