use std::{fs, path::PathBuf};

use crate::models::{ClipboardTargetProfile, StoredClipboardItem};

// Pure helpers for CF_HTML generation and mixed text/image payload reconstruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MixedPasteSegment {
    Text(String),
    Image,
}

fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn plain_text_html_fragment(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let html = escape_html(&normalized).replace('\n', "<br>");
    format!("<div>{html}</div>")
}

// Windows expects clipboard HTML in CF_HTML format with explicit byte offsets.
fn build_cf_html(fragment: &str) -> String {
    let prefix = "<html><body><!--StartFragment-->";
    let suffix = "<!--EndFragment--></body></html>";
    let html = format!("{prefix}{fragment}{suffix}");
    let empty_header =
        "Version:1.0\r\nStartHTML:0000000000\r\nEndHTML:0000000000\r\nStartFragment:0000000000\r\nEndFragment:0000000000\r\n";
    let start_html = empty_header.len();
    let start_fragment = start_html + prefix.len();
    let end_fragment = start_fragment + fragment.len();
    let end_html = start_html + html.len();
    format!(
        "Version:1.0\r\nStartHTML:{:010}\r\nEndHTML:{:010}\r\nStartFragment:{:010}\r\nEndFragment:{:010}\r\n{}",
        start_html, end_html, start_fragment, end_fragment, html
    )
}

pub(crate) fn ensure_cf_html(html: &str) -> String {
    if html.trim_start().starts_with("Version:") {
        html.to_string()
    } else {
        build_cf_html(html)
    }
}

fn encode_file_uri_component(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~'
            | b'/'
            | b':'
            | b'!'
            | b'$'
            | b'&'
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
            | b'@' => encoded.push(byte as char),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn file_uri_from_path(path: &str) -> Option<String> {
    let absolute = fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
    let raw = absolute.to_string_lossy().replace('\\', "/");
    if raw.is_empty() {
        return None;
    }
    Some(format!("file:///{}", encode_file_uri_component(&raw)))
}

// Rewrites any <img> tag source so stored HTML can point at local image files when replayed.
fn rewrite_img_tag_source(tag: &str, new_src: &str) -> String {
    let lower = tag.to_ascii_lowercase();
    let Some(src_pos) = lower.find("src") else {
        let insert_at = tag.rfind('>').unwrap_or(tag.len());
        return format!(
            "{} src=\"{}\"{}",
            &tag[..insert_at],
            escape_html(new_src),
            &tag[insert_at..]
        );
    };

    let mut cursor = src_pos + 3;
    let bytes = tag.as_bytes();
    while cursor < tag.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    if cursor >= tag.len() || bytes[cursor] != b'=' {
        let insert_at = tag.rfind('>').unwrap_or(tag.len());
        return format!(
            "{} src=\"{}\"{}",
            &tag[..insert_at],
            escape_html(new_src),
            &tag[insert_at..]
        );
    }
    cursor += 1;
    while cursor < tag.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    if cursor >= tag.len() {
        return tag.to_string();
    }

    let value_start = cursor;
    let (value_end, quoted) = match bytes[cursor] {
        b'"' => {
            let start = cursor + 1;
            let end = tag[start..]
                .find('"')
                .map(|offset| start + offset)
                .unwrap_or(tag.len());
            (end, true)
        }
        b'\'' => {
            let start = cursor + 1;
            let end = tag[start..]
                .find('\'')
                .map(|offset| start + offset)
                .unwrap_or(tag.len());
            (end, true)
        }
        _ => {
            let end = tag[cursor..]
                .find(|ch: char| ch.is_ascii_whitespace() || ch == '>')
                .map(|offset| cursor + offset)
                .unwrap_or(tag.len());
            (end, false)
        }
    };

    let (prefix_end, suffix_start) = if quoted {
        (value_start + 1, (value_end + 1).min(tag.len()))
    } else {
        (value_start, value_end)
    };

    format!(
        "{}{}{}",
        &tag[..prefix_end],
        escape_html(new_src),
        &tag[suffix_start..]
    )
}

fn rewrite_html_image_sources(html: &str, new_src: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut result = String::with_capacity(html.len() + 64);
    let mut cursor = 0usize;

    while let Some(relative_start) = lower[cursor..].find("<img") {
        let start = cursor + relative_start;
        let Some(relative_end) = lower[start..].find('>') else {
            break;
        };
        let end = start + relative_end + 1;
        result.push_str(&html[cursor..start]);
        result.push_str(&rewrite_img_tag_source(&html[start..end], new_src));
        cursor = end;
    }

    result.push_str(&html[cursor..]);
    result
}

fn cf_html_fragment(html: &str) -> &str {
    let read_offset = |label: &str| -> Option<usize> {
        let start = html.find(label)? + label.len();
        let digits = html[start..]
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>();
        digits.parse::<usize>().ok()
    };

    let start = read_offset("StartFragment:");
    let end = read_offset("EndFragment:");
    match (start, end) {
        (Some(start), Some(end))
            if start < end
                && end <= html.len()
                && html.is_char_boundary(start)
                && html.is_char_boundary(end) =>
        {
            &html[start..end]
        }
        _ => html,
    }
}

fn decode_html_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn push_text_segment(segments: &mut Vec<MixedPasteSegment>, text: &mut String) {
    if text.is_empty() {
        return;
    }

    let normalized = decode_html_entities(text)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let meaningful = normalized.chars().any(|ch| !ch.is_whitespace());
    if meaningful {
        match segments.last_mut() {
            Some(MixedPasteSegment::Text(existing)) => existing.push_str(&normalized),
            _ => segments.push(MixedPasteSegment::Text(normalized)),
        }
    }
    text.clear();
}

// Converts HTML fragments into alternating text/image segments for chat-style paste targets.
pub(crate) fn html_to_mixed_segments(html: &str) -> Vec<MixedPasteSegment> {
    let fragment = cf_html_fragment(html);
    let lower = fragment.to_ascii_lowercase();
    let mut segments = Vec::new();
    let mut text = String::new();
    let mut cursor = 0usize;

    while cursor < fragment.len() {
        let Some(relative_tag_start) = lower[cursor..].find('<') else {
            text.push_str(&fragment[cursor..]);
            break;
        };
        let tag_start = cursor + relative_tag_start;
        text.push_str(&fragment[cursor..tag_start]);

        let Some(relative_tag_end) = lower[tag_start..].find('>') else {
            text.push_str(&fragment[tag_start..]);
            break;
        };
        let tag_end = tag_start + relative_tag_end + 1;
        let tag = &lower[tag_start + 1..tag_end - 1];
        let tag = tag.trim();

        if tag.starts_with("!--") {
            cursor = tag_end;
            continue;
        }

        let tag_name = tag
            .trim_start_matches('/')
            .split(|ch: char| ch.is_ascii_whitespace() || ch == '/')
            .next()
            .unwrap_or_default();

        match tag_name {
            "img" => {
                push_text_segment(&mut segments, &mut text);
                segments.push(MixedPasteSegment::Image);
            }
            "br" | "p" | "div" | "li" | "tr" | "td" | "th" | "blockquote" | "h1" | "h2" | "h3"
            | "h4" | "h5" | "h6" => {
                if !text.ends_with('\n') {
                    text.push('\n');
                }
            }
            _ => {}
        }

        cursor = tag_end;
    }

    push_text_segment(&mut segments, &mut text);
    segments
}

fn split_string_by_char_counts(text: &str, counts: &[usize]) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let total_chars = chars.len();
    if counts.is_empty() {
        return Vec::new();
    }

    let total_weight = counts.iter().sum::<usize>().max(1);
    let mut result = Vec::with_capacity(counts.len());
    let mut consumed = 0usize;

    for (index, weight) in counts.iter().enumerate() {
        let remaining_weights = counts[index..].iter().sum::<usize>().max(1);
        let remaining_chars = total_chars.saturating_sub(consumed);
        let take = if index + 1 == counts.len() {
            remaining_chars
        } else if remaining_chars == 0 || *weight == 0 {
            0
        } else {
            ((total_chars * *weight) / total_weight)
                .max(1)
                .min(remaining_chars.saturating_sub(counts.len() - index - 1))
                .min((remaining_chars * *weight) / remaining_weights.max(1) + 1)
        };

        let end = (consumed + take).min(total_chars);
        result.push(chars[consumed..end].iter().collect());
        consumed = end;
    }

    result
}

pub(crate) fn remap_mixed_text_segments(
    segments: &[MixedPasteSegment],
    full_text: &str,
) -> Vec<MixedPasteSegment> {
    let weights: Vec<usize> = segments
        .iter()
        .filter_map(|segment| match segment {
            MixedPasteSegment::Text(text) => {
                let weight = text.chars().filter(|ch| !ch.is_whitespace()).count().max(1);
                Some(weight)
            }
            MixedPasteSegment::Image => None,
        })
        .collect();

    if weights.is_empty() {
        return segments.to_vec();
    }

    let remapped_texts = split_string_by_char_counts(full_text, &weights);
    let mut text_index = 0usize;

    segments
        .iter()
        .map(|segment| match segment {
            MixedPasteSegment::Text(_) => {
                let text = remapped_texts.get(text_index).cloned().unwrap_or_default();
                text_index += 1;
                MixedPasteSegment::Text(text)
            }
            MixedPasteSegment::Image => MixedPasteSegment::Image,
        })
        .collect()
}

pub(crate) fn build_mixed_item_html(
    item: &StoredClipboardItem,
    profile: ClipboardTargetProfile,
) -> Option<String> {
    if let Some(html) = item.html_text.as_deref().filter(|value| !value.is_empty()) {
        if let Some(image_src) = item.image_path.as_deref().and_then(file_uri_from_path) {
            return Some(ensure_cf_html(&rewrite_html_image_sources(
                html, &image_src,
            )));
        }
        return Some(ensure_cf_html(html));
    }

    let mut fragment = String::new();

    if let Some(text) = item
        .full_text
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        fragment.push_str(&plain_text_html_fragment(text));
    }

    let image_src = match profile {
        ClipboardTargetProfile::Office
        | ClipboardTargetProfile::Wps
        | ClipboardTargetProfile::Generic => item
            .image_path
            .as_deref()
            .and_then(file_uri_from_path)
            .or_else(|| item.image_data_url.clone()),
        ClipboardTargetProfile::Markdown | ClipboardTargetProfile::Chat => None,
    };

    if let Some(image_src) = image_src.filter(|value| !value.is_empty()) {
        if !fragment.is_empty() {
            fragment.push_str("<br>");
        }
        let dimension_attrs = match (item.image_width, item.image_height) {
            (Some(width), Some(height)) if width > 0 && height > 0 => format!(
                " width=\"{width}\" height=\"{height}\" style=\"width:{width}px;height:{height}px;mso-width-source:userset;mso-height-source:userset;\""
            ),
            _ => String::new(),
        };
        fragment.push_str(&format!(
            "<div><img src=\"{}\"{} alt=\"\" /></div>",
            escape_html(&image_src),
            dimension_attrs,
        ));
    }

    if fragment.is_empty() {
        None
    } else {
        Some(build_cf_html(&fragment))
    }
}
