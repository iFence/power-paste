use std::sync::Arc;

use anyhow::Result;
use tauri::AppHandle;

use crate::{
    models::{AppSettings, PlatformCapabilities, SharedState, StoredClipboardItem},
    paste_target::ResolvedPasteTarget,
};

pub(crate) trait ClipboardWriterPort {
    fn capabilities(&self) -> PlatformCapabilities;

    fn write_item(
        &self,
        app: &AppHandle,
        item: &StoredClipboardItem,
        target: &ResolvedPasteTarget,
    ) -> Result<()>;
}

pub(crate) trait PasteDispatcherPort {
    fn supports_direct_paste(&self) -> bool;

    fn prepare_target(&self, state: &Arc<SharedState>);

    fn dispatch_paste(
        &self,
        app: &AppHandle,
        state: &Arc<SharedState>,
        item: &StoredClipboardItem,
        target: &ResolvedPasteTarget,
    ) -> Result<bool>;
}

pub(crate) trait TargetTrackerPort {
    fn resolve(&self, state: &Arc<SharedState>) -> ResolvedPasteTarget;
}

pub(crate) trait SettingsRuntimePort {
    fn apply(
        &self,
        app: &AppHandle,
        state: &Arc<SharedState>,
        settings: &AppSettings,
    ) -> Result<()>;
}
