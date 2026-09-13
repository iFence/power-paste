mod backend;
mod capabilities;
mod capture_router;
mod native_writer;
mod payload;
pub(crate) mod plugin_reader;
mod plugin_writer;

use anyhow::Result;
use tauri::AppHandle;
#[cfg(target_os = "linux")]
use tauri_plugin_clipboard_next::ClipboardNextExt;

use crate::{models::StoredClipboardItem, paste_target::TargetProfile};

use self::payload::{payload_for_item, ClipboardPayload};

pub(crate) use capabilities::direct_paste_unavailable_reason;
pub(crate) use capabilities::{launch_on_startup_supported, platform_capabilities};
#[cfg(target_os = "linux")]
pub(crate) use capabilities::{
    linux_direct_paste_backend, linux_session_backend, linux_wayland_tooling_available,
    linux_x11_tooling_available,
};
pub(crate) use capture_router::capture_clipboard;
#[cfg(windows)]
pub(crate) use native_writer::write_image_to_clipboard;
#[cfg(windows)]
pub(crate) use plugin_writer::{
    write_image as write_image_with_plugin, write_text as write_text_with_plugin,
};

pub(crate) fn write_item_to_clipboard_with_profile(
    app: &AppHandle,
    item: &StoredClipboardItem,
    profile: TargetProfile,
) -> Result<ClipboardPayload> {
    let payload = payload_for_item(item);

    match backend::preferred_backend_for_payload(&payload) {
        crate::models::ClipboardBackend::Plugin => {
            let payload = degrade_plugin_only_payload(payload);
            plugin_writer::write_payload(app, &payload)
        }
        crate::models::ClipboardBackend::NativeFallback => {
            native_writer::write_payload(item, profile, &payload)
                .map(|_| payload.clone())
                .or_else(|_| plugin_writer::write_payload(app, &plugin_fallback_payload(payload)))
        }
    }
}

// 判断当前会话是否真的具备可用的系统剪贴板后端。
//
// Linux 上 tauri-plugin-clipboard-next 用 clipboard-rs 的 X11 / Wayland 两种后端，
// 两者都不可用时插件内部 `ClipboardContext::new().unwrap()` 会直接 panic；
// 局域网互传在“从剪贴板选择发送”和“收到的内容写入剪贴板”两处会调用插件，
// 因此先探测一次并缓存结果，不可用时由调用方给出稳定错误码而不是崩溃。
pub(crate) fn clipboard_backend_usable(app: &AppHandle) -> bool {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        true
    }

    #[cfg(target_os = "linux")]
    {
        static USABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

        *USABLE.get_or_init(|| {
            if !display_env_present("DISPLAY") && !display_env_present("WAYLAND_DISPLAY") {
                return false;
            }

            // 后端初始化失败时插件会 panic，这里把 panic 归为“不可用”，
            // 让互传链路退化为提示而不是中断整个接收流程。
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                app.clipboard_next().has_text().is_ok()
            }))
            .unwrap_or(false)
        })
    }
}

#[cfg(target_os = "linux")]
fn display_env_present(key: &str) -> bool {
    std::env::var_os(key).is_some_and(|value| !value.is_empty())
}

#[cfg(target_os = "macos")]
pub(crate) fn wait_for_clipboard_payload(
    app: &AppHandle,
    payload: &ClipboardPayload,
) -> Result<()> {
    let _ = app;
    let delay_ms = match payload {
        ClipboardPayload::Image { .. } => 120,
        ClipboardPayload::RichText { .. } | ClipboardPayload::Html { .. } => 80,
        _ => 40,
    };
    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    Ok(())
}

fn plugin_fallback_payload(payload: ClipboardPayload) -> ClipboardPayload {
    match payload {
        ClipboardPayload::Html { text, html } => {
            if let Some(text) = text {
                ClipboardPayload::Text { text }
            } else {
                ClipboardPayload::Html { text: None, html }
            }
        }
        ClipboardPayload::RichText { text, html, rtf } => {
            if let Some(text) = text {
                ClipboardPayload::Text { text }
            } else if let Some(html) = html {
                ClipboardPayload::Html { text: None, html }
            } else if let Some(rtf) = rtf {
                ClipboardPayload::RichText {
                    text: None,
                    html: None,
                    rtf: Some(rtf),
                }
            } else {
                ClipboardPayload::Empty
            }
        }
        ClipboardPayload::Mixed {
            text,
            html,
            png_bytes,
        } => {
            if let Some(text) = text {
                ClipboardPayload::Text { text }
            } else if let Some(html) = html {
                ClipboardPayload::Html { text: None, html }
            } else if let Some(png_bytes) = png_bytes {
                ClipboardPayload::Image { png_bytes }
            } else {
                ClipboardPayload::Empty
            }
        }
        other => other,
    }
}

fn degrade_plugin_only_payload(payload: ClipboardPayload) -> ClipboardPayload {
    if cfg!(any(windows, target_os = "macos")) {
        payload
    } else {
        match payload {
            ClipboardPayload::Mixed { .. } => plugin_fallback_payload(payload),
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::degrade_plugin_only_payload;
    use crate::clipboard::payload::ClipboardPayload;

    #[test]
    fn degrades_mixed_payload_to_single_payload_on_plugin_only_platforms() {
        let payload = ClipboardPayload::Mixed {
            text: Some("plain".into()),
            html: Some("<b>plain</b>".into()),
            png_bytes: Some(vec![1, 2, 3]),
        };

        let next = degrade_plugin_only_payload(payload);

        if cfg!(windows) || cfg!(target_os = "macos") {
            assert!(matches!(next, ClipboardPayload::Mixed { .. }));
        } else {
            assert!(matches!(next, ClipboardPayload::Text { .. }));
        }
    }

    #[test]
    fn degrades_image_only_mixed_payload_to_image_on_plugin_only_platforms() {
        let payload = ClipboardPayload::Mixed {
            text: None,
            html: None,
            png_bytes: Some(vec![1, 2, 3]),
        };

        let next = degrade_plugin_only_payload(payload);

        if cfg!(windows) || cfg!(target_os = "macos") {
            assert!(matches!(next, ClipboardPayload::Mixed { .. }));
        } else {
            assert!(matches!(next, ClipboardPayload::Image { .. }));
        }
    }

    #[test]
    fn degrades_html_only_mixed_payload_to_html_or_text() {
        let payload = ClipboardPayload::Mixed {
            text: Some("plain".into()),
            html: Some("<p>plain</p><img src=\"cid:test\" />".into()),
            png_bytes: None,
        };

        let next = degrade_plugin_only_payload(payload);

        if cfg!(windows) || cfg!(target_os = "macos") {
            assert!(matches!(next, ClipboardPayload::Mixed { .. }));
        } else {
            assert!(matches!(
                next,
                ClipboardPayload::Text { .. } | ClipboardPayload::Html { .. }
            ));
        }
    }
}
