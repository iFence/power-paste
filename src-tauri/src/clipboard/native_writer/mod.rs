use anyhow::Result;

use crate::{models::StoredClipboardItem, paste_target::TargetProfile};

use super::payload::ClipboardPayload;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub(crate) use windows::write_image_to_clipboard;
#[cfg(not(any(windows, target_os = "macos")))]
pub(crate) fn write_image_to_clipboard(_png_bytes: &[u8]) -> Result<()> {
    anyhow::bail!("unsupported_clipboard_write")
}

pub(crate) fn write_payload(
    item: &StoredClipboardItem,
    profile: TargetProfile,
    payload: &ClipboardPayload,
) -> Result<()> {
    #[cfg(windows)]
    {
        return windows::write_payload(item, profile, payload);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = item;
        let _ = profile;
        return macos::write_payload(payload);
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = item;
        let _ = profile;
        let _ = payload;
        anyhow::bail!("unsupported_clipboard_write")
    }
}
