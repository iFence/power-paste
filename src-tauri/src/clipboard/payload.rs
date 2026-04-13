use crate::models::StoredClipboardItem;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum ClipboardPayload {
    Empty,
    Text {
        text: String,
    },
    Html {
        text: Option<String>,
        html: String,
    },
    Image {
        png_bytes: Vec<u8>,
    },
    RichText {
        text: Option<String>,
        html: Option<String>,
        rtf: Option<String>,
    },
    Mixed {
        text: Option<String>,
        html: Option<String>,
        png_bytes: Option<Vec<u8>>,
    },
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(crate) fn is_image_placeholder_text(text: &str) -> bool {
    let normalized = text.trim().to_lowercase();
    matches!(normalized.as_str(), "[image]" | "image" | "[img]" | "img")
}

fn item_has_image(item: &StoredClipboardItem) -> bool {
    item.image_png
        .as_ref()
        .map(|bytes| !bytes.is_empty())
        .unwrap_or(false)
}

pub(crate) fn item_should_prefer_image_payload(item: &StoredClipboardItem) -> bool {
    item_has_image(item)
        && item
            .full_text
            .as_deref()
            .map(is_image_placeholder_text)
            .unwrap_or(false)
}

pub(crate) fn payload_for_item(item: &StoredClipboardItem) -> ClipboardPayload {
    let text = non_empty(item.full_text.as_deref());
    let html = non_empty(item.html_text.as_deref());
    let rtf = non_empty(item.rtf_text.as_deref());
    let png_bytes = item.image_png.clone().filter(|bytes| !bytes.is_empty());

    if item_should_prefer_image_payload(item) {
        return png_bytes
            .map(|png_bytes| ClipboardPayload::Image { png_bytes })
            .unwrap_or(ClipboardPayload::Empty);
    }

    match item.kind.as_str() {
        "text" => {
            if rtf.is_some() {
                ClipboardPayload::RichText { text, html, rtf }
            } else if let Some(html) = html {
                ClipboardPayload::Html { text, html }
            } else if let Some(text) = text {
                ClipboardPayload::Text { text }
            } else {
                ClipboardPayload::Empty
            }
        }
        "mixed" => match (text, html, png_bytes, rtf) {
            (text, html, png_bytes, _)
                if text.is_some() || html.is_some() || png_bytes.is_some() =>
            {
                ClipboardPayload::Mixed {
                text,
                html,
                png_bytes,
                }
            }
            (text, None, None, Some(rtf)) => ClipboardPayload::RichText {
                text,
                html: None,
                rtf: Some(rtf),
            },
            (Some(text), None, None, None) => ClipboardPayload::Text { text },
            _ => ClipboardPayload::Empty,
        },
        _ => match (text, html, rtf, png_bytes) {
            (Some(text), None, None, None) => ClipboardPayload::Text { text },
            (text, Some(html), None, None) => ClipboardPayload::Html { text, html },
            (text, html, rtf, Some(png_bytes)) => {
                if text.is_some() || html.is_some() || rtf.is_some() {
                    ClipboardPayload::Mixed {
                        text,
                        html,
                        png_bytes: Some(png_bytes),
                    }
                } else {
                    ClipboardPayload::Image { png_bytes }
                }
            }
            (text, html, Some(rtf), None) => ClipboardPayload::RichText {
                text,
                html,
                rtf: Some(rtf),
            },
            _ => ClipboardPayload::Empty,
        },
    }
}
