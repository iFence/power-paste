use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::{Instant, SystemTime},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tauri_plugin_updater::Update;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

pub(crate) const SETTINGS_FILE: &str = "settings.json";
pub(crate) const SQLITE_FILE: &str = "clipdesk.db";
pub(crate) const HISTORY_UPDATED_EVENT: &str = "history-updated";
pub(crate) const COPY_SOUND_EVENT: &str = "copy-sound";
pub(crate) const LAN_RECEIVER_STATUS_EVENT: &str = "lan-receiver-status";
pub(crate) const UPDATE_STATUS_EVENT: &str = "update-status";
pub(crate) const PANEL_LABEL: &str = "main";

#[cfg(windows)]
pub(crate) const CF_DIB: u32 = 8;

#[cfg(windows)]
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
    pub(crate) sound_enabled: bool,
    pub(crate) launch_on_startup: bool,
    pub(crate) polling_interval_ms: u64,
    pub(crate) max_history_items: usize,
    pub(crate) max_history_days: u64,
    pub(crate) max_image_bytes: usize,
    pub(crate) lan_transfer_download_dir: Option<String>,
    pub(crate) global_shortcut: String,
    pub(crate) ignored_apps: Vec<String>,
    pub(crate) locale: String,
    pub(crate) density: String,
    pub(crate) theme_mode: String,
    pub(crate) accent_color: String,
    pub(crate) tag_labels: HashMap<String, String>,
    pub(crate) window_x: Option<i32>,
    pub(crate) window_y: Option<i32>,
    pub(crate) window_width: Option<u32>,
    pub(crate) window_height: Option<u32>,
    pub(crate) main_panel_width: Option<u32>,
    pub(crate) main_panel_height: Option<u32>,
    pub(crate) main_panel_scale_factor: Option<f64>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            debug_enabled: false,
            sound_enabled: true,
            launch_on_startup: false,
            polling_interval_ms: 500,
            max_history_items: 200,
            max_history_days: 30,
            max_image_bytes: 6_000_000,
            lan_transfer_download_dir: None,
            global_shortcut: "Ctrl+Shift+V".into(),
            ignored_apps: vec!["1Password".into(), "Bitwarden".into(), "KeePassXC".into()],
            locale: "zh-CN".into(),
            density: "compact".into(),
            theme_mode: "system".into(),
            accent_color: "amber".into(),
            tag_labels: HashMap::new(),
            window_x: None,
            window_y: None,
            window_width: None,
            window_height: None,
            main_panel_width: None,
            main_panel_height: None,
            main_panel_scale_factor: None,
        }
    }
}

impl AppSettings {
    pub(crate) fn normalized(mut self) -> Self {
        self.global_shortcut = normalize_shortcut(&self.global_shortcut);
        if self.max_history_items == 0 {
            self.max_history_items = Self::default().max_history_items;
        }
        self.max_history_items = self.max_history_items.clamp(50, 10_000);
        if self.max_history_days == 0 {
            self.max_history_days = Self::default().max_history_days;
        }
        self.max_history_days = self.max_history_days.min(3650);
        self.lan_transfer_download_dir = self
            .lan_transfer_download_dir
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        self.tag_labels = self
            .tag_labels
            .into_iter()
            .filter_map(|(key, value)| {
                let normalized_key = key.trim().to_ascii_lowercase();
                if !matches!(
                    normalized_key.as_str(),
                    "red" | "orange" | "yellow" | "green" | "blue" | "purple" | "gray"
                ) {
                    return None;
                }

                let normalized_value = value.trim().to_string();
                Some((normalized_key, normalized_value))
            })
            .collect();
        self
    }
}

fn normalize_shortcut(shortcut: &str) -> String {
    shortcut
        .split('+')
        .map(|token| match token.trim().to_ascii_lowercase().as_str() {
            "meta" => "Command".to_string(),
            other => {
                if other.is_empty() {
                    String::new()
                } else {
                    token.trim().to_string()
                }
            }
        })
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
        .join("+")
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
    pub(crate) image_original_bytes: Option<Vec<u8>>,
    pub(crate) image_original_mime: Option<String>,
    pub(crate) image_preview_png: Option<Vec<u8>>,
    pub(crate) image_width: Option<u32>,
    pub(crate) image_height: Option<u32>,
    pub(crate) source_app: Option<String>,
    pub(crate) source_icon_data_url: Option<String>,
    pub(crate) hash: String,
    pub(crate) pinned: bool,
    pub(crate) favorite: bool,
    pub(crate) tag_colors: Vec<String>,
}

impl StoredClipboardItem {
    pub(crate) fn image_data_url(&self) -> Option<String> {
        self.image_preview_png
            .as_ref()
            .filter(|bytes| !bytes.is_empty())
            .or(self.image_png.as_ref().filter(|bytes| !bytes.is_empty()))
            .map(|bytes| format!("data:image/png;base64,{}", BASE64.encode(bytes)))
    }

    pub(crate) fn image_display_byte_size(&self) -> Option<usize> {
        self.image_original_bytes
            .as_ref()
            .filter(|bytes| !bytes.is_empty())
            .or(self.image_png.as_ref().filter(|bytes| !bytes.is_empty()))
            .map(Vec::len)
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
    pub(crate) image_byte_size: Option<usize>,
    pub(crate) image_width: Option<u32>,
    pub(crate) image_height: Option<u32>,
    pub(crate) source_app: Option<String>,
    pub(crate) source_icon_data_url: Option<String>,
    pub(crate) pinned: bool,
    pub(crate) favorite: bool,
    pub(crate) tag_colors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClipboardHistoryPageDto {
    pub(crate) items: Vec<ClipboardItemDto>,
    pub(crate) total_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct StoragePaths {
    pub(crate) db_path: PathBuf,
    pub(crate) settings_path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WindowSizePayload {
    pub(crate) width: u32,
    pub(crate) height: u32,
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
    pub(crate) last_sound_event_at: Option<Instant>,
    #[cfg(windows)]
    pub(crate) last_target_window: Option<HwndRaw>,
    #[cfg(target_os = "linux")]
    pub(crate) last_target_window_id: Option<String>,
    #[cfg(target_os = "macos")]
    pub(crate) last_target_app_bundle_id: Option<String>,
    #[cfg(target_os = "macos")]
    pub(crate) last_target_app_name: Option<String>,
}

pub(crate) struct SharedState {
    pub(crate) paths: StoragePaths,
    pub(crate) settings: Arc<Mutex<AppSettings>>,
    pub(crate) history_store: Arc<Mutex<crate::repository::SqliteHistoryStore>>,
    pub(crate) monitor: Arc<Mutex<MonitorState>>,
    pub(crate) debug_context_menu_enabled: Arc<AtomicBool>,
    #[cfg(target_os = "macos")]
    pub(crate) macos_direct_paste_permission_verified: Arc<AtomicBool>,
    pub(crate) update_status: Arc<Mutex<UpdateStatus>>,
    pub(crate) pending_update: Arc<Mutex<Option<Update>>>,
    pub(crate) update_debug_override: Arc<Mutex<Option<UpdateStatus>>>,
    pub(crate) lan_receiver: Arc<Mutex<Option<LanReceiverSession>>>,
}

#[derive(Debug)]
pub(crate) struct LanReceiverSession {
    pub(crate) url: String,
    pub(crate) qr_svg: String,
    pub(crate) ip: String,
    pub(crate) port: u16,
    pub(crate) token: String,
    pub(crate) expires_at: Option<SystemTime>,
    pub(crate) stop_requested: Arc<AtomicBool>,
    pub(crate) last_status: Option<LanReceiverStatus>,
    pub(crate) last_phone_seen: Option<SystemTime>,
    pub(crate) last_activity: SystemTime,
    pub(crate) messages: Vec<LanTransferMessage>,
    pub(crate) files: HashMap<String, LanTransferFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanReceiverStateDto {
    pub(crate) running: bool,
    pub(crate) url: Option<String>,
    pub(crate) qr_svg: Option<String>,
    pub(crate) ip: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) token: Option<String>,
    pub(crate) expires_at: Option<u64>,
    pub(crate) last_status: Option<LanReceiverStatus>,
    pub(crate) connected_devices: usize,
    pub(crate) messages: Vec<LanTransferMessageDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanReceiverStatus {
    pub(crate) kind: String,
    pub(crate) message: String,
    pub(crate) received_kind: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct LanTransferFile {
    pub(crate) file_name: String,
    pub(crate) mime_type: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct LanTransferMessage {
    pub(crate) id: String,
    pub(crate) sender: String,
    pub(crate) kind: String,
    pub(crate) text: Option<String>,
    pub(crate) file_name: Option<String>,
    pub(crate) mime_type: Option<String>,
    pub(crate) size: Option<usize>,
    pub(crate) image_data_url: Option<String>,
    pub(crate) download_url: Option<String>,
    pub(crate) local_path: Option<PathBuf>,
    pub(crate) created_at: u64,
    pub(crate) status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanTransferMessageDto {
    pub(crate) id: String,
    pub(crate) sender: String,
    pub(crate) kind: String,
    pub(crate) text: Option<String>,
    pub(crate) file_name: Option<String>,
    pub(crate) mime_type: Option<String>,
    pub(crate) size: Option<usize>,
    pub(crate) image_data_url: Option<String>,
    pub(crate) download_url: Option<String>,
    pub(crate) has_local_file: bool,
    pub(crate) created_at: u64,
    pub(crate) status: String,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateDebugStatePayload {
    pub(crate) status: String,
    pub(crate) latest_version: Option<String>,
    pub(crate) body: Option<String>,
    pub(crate) published_at: Option<String>,
    pub(crate) downloaded_bytes: Option<u64>,
    pub(crate) content_length: Option<u64>,
    pub(crate) error: Option<String>,
}

#[derive(Debug)]
pub(crate) enum CapturedClipboard {
    Text {
        text: String,
        html_text: Option<String>,
        rtf_text: Option<String>,
        hash: String,
    },
    Link {
        text: String,
        html_text: Option<String>,
        rtf_text: Option<String>,
        hash: String,
    },
    Image {
        png_bytes: Vec<u8>,
        original_bytes: Option<Vec<u8>>,
        original_mime: Option<String>,
        hash: String,
        preview: String,
        image_width: u32,
        image_height: u32,
    },
    Mixed {
        text: String,
        html_text: Option<String>,
        rtf_text: Option<String>,
        png_bytes: Option<Vec<u8>>,
        hash: String,
        image_width: u32,
        image_height: u32,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForegroundAppResult {
    pub(crate) process_name: String,
    pub(crate) display_name: String,
    pub(crate) icon_png_base64: Option<String>,
    pub(crate) app_path: Option<String>,
}
