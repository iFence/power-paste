pub(crate) fn mobile_page(max_image_bytes: usize, accent_color: &str, poll_ms: u64) -> String {
    let (accent_primary, accent_strong, accent_text) = mobile_accent_palette(accent_color);
    include_str!("lan_receiver_mobile_page.html")
        .replace("__MAX_IMAGE_BYTES__", &max_image_bytes.to_string())
        .replace("__POLL_MS__", &poll_ms.to_string())
        .replace("__ACCENT_PRIMARY__", accent_primary)
        .replace("__ACCENT_STRONG__", accent_strong)
        .replace("__ACCENT_TEXT__", accent_text)
}

fn mobile_accent_palette(accent_color: &str) -> (&'static str, &'static str, &'static str) {
    match accent_color {
        "ocean" => ("#68b6ff", "#3e7fe6", "#0d1a2a"),
        "jade" => ("#62d6b1", "#2f9f83", "#0c1f1b"),
        "rose" => ("#f08db0", "#d45a86", "#2b1019"),
        _ => ("#f0b35f", "#dd8648", "#24160d"),
    }
}
