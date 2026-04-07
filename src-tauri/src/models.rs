use std::{
    fs,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Instant,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tauri_plugin_updater::Update;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

pub(crate) const SETTINGS_FILE: &str = "settings.json";
pub(crate) const SQLITE_FILE: &str = "clipdesk.db";
pub(crate) const HISTORY_UPDATED_EVENT: &str = "history-updated";
pub(crate) const UPDATE_STATUS_EVENT: &str = "update-status";
pub(crate) const PANEL_LABEL: &str = "main";
pub(crate) const DEBUG_CONTEXT_MENU_INIT_SCRIPT: &str = r#"
;(() => {
  const state = (window.__CLIPDESK_DEBUG_GUARD__ ??= { allowContextMenu: false });
  const blockContextMenu = (event) => {
    if (state.allowContextMenu) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    if (typeof event.stopImmediatePropagation === "function") {
      event.stopImmediatePropagation();
    }
  };

  window.addEventListener("contextmenu", blockContextMenu, true);
  document.addEventListener("contextmenu", blockContextMenu, true);
})();
"#;

pub(crate) const CF_DIB: u32 = 8;

pub(crate) type HwndRaw = isize;

#[derive(Debug, thiserror::Error)]
pub(crate) enum AppError {
    #[error("{0}")]
    Message(String),
}

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        Self::Message(value.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlatformCapabilities {
    pub(crate) platform: String,
    pub(crate) supports_clipboard_read: bool,
    pub(crate) supports_clipboard_watch: bool,
    pub(crate) supports_text_write: bool,
    pub(crate) supports_html_write: bool,
    pub(crate) supports_image_write: bool,
    pub(crate) supports_direct_paste: bool,
    pub(crate) supports_mixed_replay: bool,
    pub(crate) supports_launch_on_startup: bool,
    pub(crate) preferred_clipboard_backend: &'static str,
    pub(crate) clipboard_write_strategy: &'static str,
    pub(crate) direct_paste_strategy: &'static str,
    pub(crate) mixed_replay_strategy: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClipboardBackend {
    Plugin,
    NativeFallback,
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppSettings {
    pub(crate) debug_enabled: bool,
    pub(crate) launch_on_startup: bool,
    pub(crate) polling_interval_ms: u64,
    pub(crate) max_history_items: usize,
    pub(crate) max_image_bytes: usize,
    pub(crate) global_shortcut: String,
    pub(crate) ignored_apps: Vec<String>,
    pub(crate) locale: String,
    pub(crate) density: String,
    pub(crate) theme_mode: String,
    pub(crate) accent_color: String,
    pub(crate) window_x: Option<i32>,
    pub(crate) window_y: Option<i32>,
    pub(crate) window_width: Option<u32>,
    pub(crate) window_height: Option<u32>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            debug_enabled: false,
            launch_on_startup: false,
            polling_interval_ms: 500,
            max_history_items: 200,
            max_image_bytes: 6_000_000,
            global_shortcut: "Ctrl+Shift+V".into(),
            ignored_apps: vec!["1Password".into(), "Bitwarden".into(), "KeePassXC".into()],
            locale: "zh-CN".into(),
            density: "compact".into(),
            theme_mode: "system".into(),
            accent_color: "amber".into(),
            window_x: None,
            window_y: None,
            window_width: None,
            window_height: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredClipboardItem {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) created_at: String,
    pub(crate) pinned_at: Option<String>,
    pub(crate) preview: String,
    pub(crate) full_text: Option<String>,
    pub(crate) html_text: Option<String>,
    pub(crate) rtf_text: Option<String>,
    pub(crate) image_png: Option<Vec<u8>>,
    pub(crate) image_width: Option<u32>,
    pub(crate) image_height: Option<u32>,
    pub(crate) source_app: Option<String>,
    pub(crate) source_icon_data_url: Option<String>,
    pub(crate) hash: String,
    pub(crate) pinned: bool,
    pub(crate) favorite: bool,
}

impl StoredClipboardItem {
    pub(crate) fn image_data_url(&self) -> Option<String> {
        self.image_png
            .as_ref()
            .filter(|bytes| !bytes.is_empty())
            .map(|bytes| format!("data:image/png;base64,{}", BASE64.encode(bytes)))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClipboardItemDto {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) created_at: String,
    pub(crate) preview: String,
    pub(crate) full_text: Option<String>,
    pub(crate) image_data_url: Option<String>,
    pub(crate) image_width: Option<u32>,
    pub(crate) image_height: Option<u32>,
    pub(crate) source_app: Option<String>,
    pub(crate) source_icon_data_url: Option<String>,
    pub(crate) pinned: bool,
    pub(crate) favorite: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct StoragePaths {
    pub(crate) db_path: PathBuf,
    pub(crate) settings_path: PathBuf,
}

impl StoragePaths {
    pub(crate) fn new(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;

        Ok(Self {
            db_path: root.join(SQLITE_FILE),
            settings_path: root.join(SETTINGS_FILE),
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct MonitorState {
    pub(crate) last_seen_hash: Option<String>,
    pub(crate) suppress_hash: Option<String>,
    pub(crate) suppress_until: Option<Instant>,
    #[cfg(windows)]
    pub(crate) last_target_window: Option<HwndRaw>,
    #[cfg(target_os = "macos")]
    pub(crate) last_target_app_bundle_id: Option<String>,
    #[cfg(target_os = "macos")]
    pub(crate) last_target_app_name: Option<String>,
}

pub(crate) struct SharedState {
    pub(crate) paths: StoragePaths,
    pub(crate) settings: Arc<Mutex<AppSettings>>,
    pub(crate) history_store: Arc<Mutex<crate::repository::SqliteHistoryStore>>,
    pub(crate) history: Arc<Mutex<Vec<StoredClipboardItem>>>,
    pub(crate) monitor: Arc<Mutex<MonitorState>>,
    pub(crate) debug_context_menu_enabled: Arc<AtomicBool>,
    pub(crate) update_status: Arc<Mutex<UpdateStatus>>,
    pub(crate) pending_update: Arc<Mutex<Option<Update>>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateStatus {
    pub(crate) status: String,
    pub(crate) current_version: String,
    pub(crate) latest_version: Option<String>,
    pub(crate) body: Option<String>,
    pub(crate) published_at: Option<String>,
    pub(crate) downloaded_bytes: Option<u64>,
    pub(crate) content_length: Option<u64>,
    pub(crate) error: Option<String>,
}

impl UpdateStatus {
    pub(crate) fn idle(current_version: String) -> Self {
        Self {
            status: "idle".into(),
            current_version,
            latest_version: None,
            body: None,
            published_at: None,
            downloaded_bytes: None,
            content_length: None,
            error: None,
        }
    }
}

#[derive(Debug)]
pub(crate) enum CapturedClipboard {
    Text {
        text: String,
        html_text: Option<String>,
        rtf_text: Option<String>,
        hash: String,
    },
    Image {
        png_bytes: Vec<u8>,
        hash: String,
        preview: String,
        image_width: u32,
        image_height: u32,
    },
    Mixed {
        text: String,
        html_text: Option<String>,
        rtf_text: Option<String>,
        png_bytes: Vec<u8>,
        hash: String,
        image_width: u32,
        image_height: u32,
    },
}


#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForegroundAppResult {
    pub(crate) process_name: String,
    pub(crate) display_name: String,
    pub(crate) icon_png_base64: Option<String>,
    pub(crate) app_path: Option<String>,
}

