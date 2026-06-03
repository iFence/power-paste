use crate::storage::preview_text;

const MIN_STANDALONE_SECRET_LEN: usize = 16;
const SENSITIVE_KEYWORDS: [&str; 14] = [
    "password",
    "passwd",
    "pwd",
    "secret",
    "token",
    "api_key",
    "apikey",
    "access_key",
    "client_secret",
    "private_key",
    "authorization",
    "密码",
    "口令",
    "密钥",
];
const KNOWN_SECRET_PREFIXES: [&str; 14] = [
    "bearer ",
    "sk-",
    "sk_live_",
    "sk_test_",
    "rk_live_",
    "ghp_",
    "github_pat_",
    "glpat-",
    "xoxb-",
    "xoxp-",
    "xapp-",
    "ya29.",
    "akia",
    "asia",
];

#[derive(Debug, Clone, Default)]
pub(crate) struct SensitiveTextMask {
    pub(crate) is_sensitive: bool,
    pub(crate) masked_preview: Option<String>,
    pub(crate) masked_full_text: Option<String>,
}

pub(crate) fn build_sensitive_text_mask(
    kind: &str,
    preview: &str,
    full_text: Option<&str>,
) -> SensitiveTextMask {
    if !matches!(kind, "text" | "mixed") {
        return SensitiveTextMask::default();
    }

    let source = full_text.unwrap_or(preview);
    let masked_full_text = match mask_sensitive_text(source) {
        Some(value) if value != source => value,
        _ => return SensitiveTextMask::default(),
    };

    SensitiveTextMask {
        is_sensitive: true,
        masked_preview: Some(preview_text(&masked_full_text)),
        masked_full_text: full_text.map(|_| masked_full_text),
    }
}

fn mask_sensitive_text(text: &str) -> Option<String> {
    if looks_like_private_key_block(text) {
        return Some(mask_trimmed_range(text, &mask_visible_token(text.trim())));
    }

    if let Some(masked) = mask_line_by_line(text) {
        return Some(masked);
    }

    let trimmed = text.trim();
    if looks_like_standalone_secret(trimmed) {
        return Some(mask_trimmed_range(text, &mask_visible_token(trimmed)));
    }

    None
}

fn mask_line_by_line(text: &str) -> Option<String> {
    let mut changed = false;
    let mut masked = String::with_capacity(text.len());

    for segment in text.split_inclusive('\n') {
        let (line, line_ending) = segment
            .strip_suffix('\n')
            .map(|value| (value, "\n"))
            .unwrap_or((segment, ""));

        let next_line = mask_sensitive_line(line).unwrap_or_else(|| line.to_string());
        if next_line != line {
            changed = true;
        }
        masked.push_str(&next_line);
        masked.push_str(line_ending);
    }

    changed.then_some(masked)
}

fn mask_sensitive_line(line: &str) -> Option<String> {
    if let Some(index) = find_case_insensitive(line, "bearer ") {
        let start = index + "bearer ".len();
        let token = line[start..].trim();
        if looks_like_secret_value(token, false) {
            return Some(format!("{}{}", &line[..start], mask_visible_token(token)));
        }
    }

    let separator_index = line.find(['=', ':'])?;
    let left = &line[..separator_index];
    if !contains_sensitive_keyword(left) {
        return None;
    }

    let right = &line[separator_index + 1..];
    let trimmed_start_len = right.len().saturating_sub(right.trim_start().len());
    let value = right.trim();
    if value.is_empty() {
        return None;
    }

    let masked_value = if looks_like_private_key_block(value) {
        mask_visible_token(value)
    } else if value.to_ascii_lowercase().starts_with("bearer ") {
        let token = value[7..].trim();
        if !looks_like_secret_value(token, false) {
            return None;
        }
        format!("Bearer {}", mask_visible_token(token))
    } else if let Some(stripped) = strip_wrapping_quotes(value) {
        if !looks_like_secret_value(stripped, true) {
            return None;
        }
        wrap_with_original_quotes(value, &mask_visible_token(stripped))
    } else {
        if !looks_like_secret_value(value, true) {
            return None;
        }
        mask_visible_token(value)
    };

    Some(format!(
        "{}{}{}{}",
        left,
        &line[separator_index..=separator_index],
        &right[..trimmed_start_len],
        masked_value
    ))
}

fn looks_like_private_key_block(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("-----begin") && lower.contains("private key-----")
}

fn looks_like_standalone_secret(text: &str) -> bool {
    if text.len() < MIN_STANDALONE_SECRET_LEN || text.contains(char::is_whitespace) {
        return false;
    }
    if looks_like_url(text) {
        return false;
    }
    if looks_like_jwt(text) {
        return true;
    }
    if looks_like_known_secret_prefix(text) {
        return true;
    }
    if looks_like_hex_secret(text) {
        return true;
    }
    if !text.chars().all(is_secret_char) {
        return false;
    }

    let mut has_lower = false;
    let mut has_upper = false;
    let mut has_digit = false;
    let mut has_symbol = false;

    for ch in text.chars() {
        if ch.is_ascii_lowercase() {
            has_lower = true;
        } else if ch.is_ascii_uppercase() {
            has_upper = true;
        } else if ch.is_ascii_digit() {
            has_digit = true;
        } else {
            has_symbol = true;
        }
    }

    let category_count = [has_lower, has_upper, has_digit, has_symbol]
        .into_iter()
        .filter(|value| *value)
        .count();
    category_count >= 3 || (text.len() >= 24 && category_count >= 2 && (has_digit || has_symbol))
}

fn looks_like_secret_value(value: &str, relax_for_keyword: bool) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    if looks_like_private_key_block(trimmed) {
        return true;
    }
    if looks_like_standalone_secret(trimmed) {
        return true;
    }
    if let Some(rest) = trimmed
        .to_ascii_lowercase()
        .strip_prefix("bearer ")
        .map(ToString::to_string)
    {
        return looks_like_standalone_secret(rest.trim());
    }

    relax_for_keyword && trimmed.chars().count() >= 4
}

fn looks_like_known_secret_prefix(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    KNOWN_SECRET_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix) && text.len() >= prefix.len() + 8)
}

fn looks_like_hex_secret(text: &str) -> bool {
    text.len() >= 32 && text.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn looks_like_jwt(text: &str) -> bool {
    let segments: Vec<&str> = text.split('.').collect();
    if segments.len() != 3 {
        return false;
    }

    segments
        .iter()
        .all(|segment| segment.len() >= 6 && segment.chars().all(is_base64_url_char))
}

fn looks_like_url(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn contains_sensitive_keyword(text: &str) -> bool {
    let lower = text.trim().to_ascii_lowercase();
    SENSITIVE_KEYWORDS
        .iter()
        .any(|keyword| lower.contains(keyword))
}

fn strip_wrapping_quotes(value: &str) -> Option<&str> {
    if value.len() < 2 {
        return None;
    }

    let starts_with_double = value.starts_with('"') && value.ends_with('"');
    let starts_with_single = value.starts_with('\'') && value.ends_with('\'');
    if starts_with_double || starts_with_single {
        return Some(&value[1..value.len() - 1]);
    }

    None
}

fn wrap_with_original_quotes(original: &str, inner: &str) -> String {
    let first = original.chars().next().unwrap_or('"');
    let last = original.chars().last().unwrap_or('"');
    format!("{first}{inner}{last}")
}

fn mask_trimmed_range(text: &str, masked_value: &str) -> String {
    let start = text.find(|ch: char| !ch.is_whitespace()).unwrap_or(0);
    let end = text
        .rfind(|ch: char| !ch.is_whitespace())
        .map(|index| index + 1)
        .unwrap_or(text.len());

    format!("{}{}{}", &text[..start], masked_value, &text[end..])
}

fn mask_visible_token(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let len = chars.len();
    if len <= 3 {
        return "***".into();
    }
    if len <= 6 {
        return format!("{}***{}", chars[0], chars[len - 1]);
    }
    if len <= 9 {
        let prefix: String = chars.iter().take(3).collect();
        let suffix: String = chars.iter().skip(len - 2).collect();
        return format!("{prefix}***{suffix}");
    }

    let prefix: String = chars.iter().take(5).collect();
    let suffix: String = chars.iter().skip(len - 2).collect();
    format!("{prefix}***{suffix}")
}

fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    let lower_haystack = haystack.to_ascii_lowercase();
    lower_haystack.find(&needle.to_ascii_lowercase())
}

fn is_secret_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | '+' | '=' | '~')
}

fn is_base64_url_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '=')
}

#[cfg(test)]
mod tests {
    use super::{build_sensitive_text_mask, mask_sensitive_text};

    #[test]
    fn masks_password_assignment_text() {
        let masked = mask_sensitive_text("password=SuperSecret123!")
            .expect("should mask password assignment");
        assert_eq!(masked, "password=Super***3!");
    }

    #[test]
    fn masks_bearer_token_text() {
        let masked =
            mask_sensitive_text("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.abc12345.def67890")
                .expect("should mask bearer token");
        assert_eq!(masked, "Bearer eyJhb***90");
    }

    #[test]
    fn masks_standalone_secret_text() {
        let masked = build_sensitive_text_mask(
            "text",
            "tp-cbs7fccxc3qetc2axwabzgw9kah62xrsz9nxy0w6",
            Some("tp-cbs7fccxc3qetc2axwabzgw9kah62xrsz9nxy0w6"),
        );
        assert!(masked.is_sensitive);
        assert_eq!(masked.masked_preview.as_deref(), Some("tp-cb***w6"));
        assert_eq!(masked.masked_full_text.as_deref(), Some("tp-cb***w6"));
    }

    #[test]
    fn ignores_normal_text() {
        let masked = build_sensitive_text_mask("text", "hello world", Some("hello world"));
        assert!(!masked.is_sensitive);
        assert!(masked.masked_preview.is_none());
        assert!(masked.masked_full_text.is_none());
    }
}
