use anyhow::Result;

use crate::{models::StoredClipboardItem, paste_target::TargetProfile};

use super::payload::ClipboardPayload;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
pub(crate) use macos::write_image_to_clipboard;
#[cfg(windows)]
pub(crate) use windows::write_image_to_clipboard;

pub(crate) fn write_payload(
    item: &StoredClipboardItem,
    profile: TargetProfile,
    payload: &ClipboardPayload,
) -> Result<()> {
    #[cfg(windows)]
    {
        let _ = payload;
        return windows::write_mixed_payload(item, profile);
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
