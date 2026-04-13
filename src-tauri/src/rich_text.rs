fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

pub(crate) fn normalize_clipboard_text(text: String) -> Option<String> {
    let normalized = normalize_line_endings(&text);
    (!normalized.trim().is_empty()).then_some(normalized)
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

fn decode_html_entity(entity: &str) -> Option<String> {
    match entity {
        "nbsp" | "#160" => Some(" ".into()),
        "amp" => Some("&".into()),
        "lt" => Some("<".into()),
        "gt" => Some(">".into()),
        "quot" => Some("\"".into()),
        "apos" | "#39" => Some("'".into()),
        _ if entity.starts_with("#x") || entity.starts_with("#X") => {
            u32::from_str_radix(&entity[2..], 16)
                .ok()
                .and_then(char::from_u32)
                .map(|ch| ch.to_string())
        }
        _ if entity.starts_with('#') => entity[1..]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map(|ch| ch.to_string()),
        _ => None,
    }
}

fn push_space(output: &mut String) {
    if output.is_empty() || output.ends_with([' ', '\n']) {
        return;
    }
    output.push(' ');
}

fn push_line_break(output: &mut String) {
    while output.ends_with(' ') {
        output.pop();
    }
    if output.ends_with('\n') {
        return;
    }
    output.push('\n');
}

fn block_level_tag(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "br"
            | "caption"
            | "dd"
            | "div"
            | "dl"
            | "dt"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "tbody"
            | "td"
            | "tfoot"
            | "th"
            | "thead"
            | "tr"
            | "ul"
    )
}

pub(crate) fn html_to_plain_text(html: &str) -> Option<String> {
    let fragment = cf_html_fragment(html);
    let mut output = String::new();
    let mut chars = fragment.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '<' => {
                let mut tag = String::new();
                let mut found_end = false;
                for next in chars.by_ref() {
                    if next == '>' {
                        found_end = true;
                        break;
                    }
                    tag.push(next);
                }
                if !found_end {
                    output.push('<');
                    output.push_str(&tag);
                    break;
                }

                let tag = tag.trim();
                if tag.starts_with("!--") {
                    continue;
                }

                let tag_name = tag
                    .trim_start_matches('/')
                    .split(|value: char| value.is_ascii_whitespace() || value == '/')
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase();

                if block_level_tag(&tag_name) {
                    push_line_break(&mut output);
                }
            }
            '&' => {
                let mut entity = String::new();
                let mut found_end = false;
                while let Some(next) = chars.peek().copied() {
                    chars.next();
                    if next == ';' {
                        found_end = true;
                        break;
                    }
                    if entity.len() >= 16 || next.is_ascii_whitespace() || next == '<' {
                        output.push('&');
                        output.push_str(&entity);
                        output.push(next);
                        found_end = true;
                        break;
                    }
                    entity.push(next);
                }

                if !found_end {
                    output.push('&');
                    output.push_str(&entity);
                    break;
                }

                if let Some(decoded) = decode_html_entity(&entity) {
                    if decoded == " " {
                        push_space(&mut output);
                    } else {
                        output.push_str(&decoded);
                    }
                } else if !entity.is_empty() {
                    output.push('&');
                    output.push_str(&entity);
                    output.push(';');
                }
            }
            value if value.is_whitespace() => push_space(&mut output),
            _ => output.push(ch),
        }
    }

    let normalized = output
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    normalize_clipboard_text(normalized)
}

pub(crate) fn html_contains_image_content(html: &str) -> bool {
    let fragment = cf_html_fragment(html);
    let lower = fragment.to_ascii_lowercase();

    lower.contains("<img")
        || lower.contains("data:image/")
        || lower.contains("content-type:image/")
        || lower.contains("content-type: image/")
}

pub(crate) fn first_html_image_src(html: &str) -> Option<String> {
    let fragment = cf_html_fragment(html);
    let lower = fragment.to_ascii_lowercase();
    let start = lower.find("<img")?;
    let end = lower[start..].find('>').map(|offset| start + offset + 1)?;
    let tag = &fragment[start..end];
    let tag_lower = &lower[start..end];
    let src_pos = tag_lower.find("src")?;

    let mut cursor = src_pos + 3;
    let bytes = tag.as_bytes();
    while cursor < tag.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    if cursor >= tag.len() || bytes[cursor] != b'=' {
        return None;
    }
    cursor += 1;
    while cursor < tag.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    if cursor >= tag.len() {
        return None;
    }

    let value = match bytes[cursor] {
        b'"' => {
            let start = cursor + 1;
            let end = tag[start..].find('"').map(|offset| start + offset)?;
            &tag[start..end]
        }
        b'\'' => {
            let start = cursor + 1;
            let end = tag[start..].find('\'').map(|offset| start + offset)?;
            &tag[start..end]
        }
        _ => {
            let end = tag[cursor..]
                .find(|ch: char| ch.is_ascii_whitespace() || ch == '>')
                .map(|offset| cursor + offset)
                .unwrap_or(tag.len());
            &tag[cursor..end]
        }
    };

    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let lower_value = value.to_ascii_lowercase();
    if lower_value.starts_with("data:image/")
        || lower_value.starts_with("http://")
        || lower_value.starts_with("https://")
        || lower_value.starts_with("file://")
    {
        Some(value.to_string())
    } else if looks_like_windows_file_path(value) {
        Some(windows_path_to_file_url(value))
    } else if value.starts_with('/') {
        Some(format!("file://{value}"))
    } else {
        None
    }
}

fn looks_like_windows_file_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() > 2
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}

fn windows_path_to_file_url(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    format!("file:///{normalized}")
}

fn count_html_tag_signals(lower: &str) -> usize {
    [
        "<p",
        "<div",
        "<span",
        "<h1",
        "<h2",
        "<h3",
        "<h4",
        "<h5",
        "<h6",
        "<ul",
        "<ol",
        "<li",
        "<table",
        "<tr",
        "<td",
        "<th",
        "<blockquote",
        "<a ",
        "<strong",
        "<em",
        "<b>",
        "<i>",
    ]
    .iter()
    .filter(|tag| lower.contains(**tag))
    .count()
}

fn looks_like_browser_rich_html(text: &str) -> bool {
    let trimmed = text.trim();
    if !(trimmed.starts_with('<') || trimmed.starts_with("Version:")) || !trimmed.contains('>') {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();

    if [
        "<!doctype",
        "<html",
        "<head",
        "<body",
        "<meta",
        "version:1.0",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
    {
        return true;
    }

    if [
        "style=",
        "class=",
        "font-family:",
        "background-color:",
        "rgb(",
    ]
    .iter()
    .any(|signal| lower.contains(signal))
    {
        return true;
    }

    count_html_tag_signals(&lower) >= 2 && lower.contains("</")
}

pub(crate) fn normalize_rich_text_payload(
    text: Option<String>,
    html: Option<String>,
) -> (Option<String>, Option<String>) {
    let text = text.and_then(normalize_clipboard_text);
    let html = html.and_then(normalize_clipboard_text);

    if let Some(html_text) = html {
        if let Some(plain_text) = html_to_plain_text(&html_text) {
            let should_replace_text = text
                .as_deref()
                .map(|value| {
                    looks_like_browser_rich_html(value) || value.trim() == html_text.trim()
                })
                .unwrap_or(true);

            if should_replace_text {
                return (Some(plain_text), Some(html_text));
            }
        }

        return (text, Some(html_text));
    }

    if let Some(text_value) = text {
        if looks_like_browser_rich_html(&text_value) {
            if let Some(plain_text) = html_to_plain_text(&text_value) {
                return (Some(plain_text), Some(text_value));
            }
        }

        return (Some(text_value), None);
    }

    (None, None)
}

#[cfg(test)]
mod tests {
    use super::{
        first_html_image_src, html_contains_image_content, html_to_plain_text,
        normalize_rich_text_payload,
    };

    #[test]
    fn extracts_plain_text_from_browser_html_fragment() {
        let html = "<meta charset='utf-8'><h3 style=\"font-size: 1.25em; color: rgb(209, 215, 224);\">🚀 优化改进</h3><p>更顺滑</p>";

        let plain_text = html_to_plain_text(html).expect("plain text");

        assert_eq!(plain_text, "🚀 优化改进\n更顺滑");
    }

    #[test]
    fn replaces_raw_html_text_when_html_payload_exists() {
        let raw_html = "<meta charset='utf-8'><h3 style=\"font-size: 1.25em;\">🚀 优化改进</h3>";

        let (text, html) =
            normalize_rich_text_payload(Some(raw_html.into()), Some(raw_html.into()));

        assert_eq!(text.as_deref(), Some("🚀 优化改进"));
        assert_eq!(html.as_deref(), Some(raw_html));
    }

    #[test]
    fn upgrades_strong_html_text_to_rich_text_payload() {
        let raw_html = "<meta charset='utf-8'><h3 style=\"font-size: 1.25em;\">🚀 优化改进</h3>";

        let (text, html) = normalize_rich_text_payload(Some(raw_html.into()), None);

        assert_eq!(text.as_deref(), Some("🚀 优化改进"));
        assert_eq!(html.as_deref(), Some(raw_html));
    }

    #[test]
    fn keeps_plain_code_snippet_as_text() {
        let snippet = "<div>Hello</div>";

        let (text, html) = normalize_rich_text_payload(Some(snippet.into()), None);

        assert_eq!(text.as_deref(), Some(snippet));
        assert_eq!(html, None);
    }

    #[test]
    fn detects_image_tags_inside_html() {
        assert!(html_contains_image_content(
            "<p>hello</p><img src=\"data:image/png;base64,abc\" />"
        ));
    }

    #[test]
    fn extracts_first_supported_image_src_from_html() {
        assert_eq!(
            first_html_image_src("<p>hello</p><img src=\"data:image/png;base64,abc\" /><img src=\"https://example.com/b.png\" />"),
            Some("data:image/png;base64,abc".into())
        );
    }

    #[test]
    fn converts_windows_file_path_image_src_to_file_url() {
        assert_eq!(
            first_html_image_src("<img src=\"C:\\Users\\yulei\\demo.png\" />"),
            Some("file:///C:/Users/yulei/demo.png".into())
        );
    }
}
