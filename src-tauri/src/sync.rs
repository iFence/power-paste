use std::{
    collections::HashMap,
    future::Future,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use reqwest::{header::ETAG, Client, Method, StatusCode, Url};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::models::{
    AppError, DeletedClipboardItem, SharedState, StoredClipboardItem, WebdavCredentialPayload,
    WebdavSyncSettings, WebdavSyncStatusDto, WEBDAV_SYNC_STATUS_EVENT,
};

const CREDENTIAL_SERVICE: &str = "power-paste-webdav";
const SYNC_ROOT_DIR: &str = ".power-paste-sync";
const ITEMS_DIR: &str = "items";
const MANIFEST_FILE: &str = "manifest.json";
const MANIFEST_VERSION: u32 = 1;
const WEBDAV_RETRY_ATTEMPTS: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
struct WebdavManifest {
    version: u32,
    items: HashMap<String, WebdavManifestEntry>,
    deleted: HashMap<String, WebdavManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebdavManifestEntry {
    sync_updated_at: String,
    sync_device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncedClipboardItem {
    id: String,
    kind: String,
    created_at: String,
    pinned_at: Option<String>,
    preview: String,
    full_text: Option<String>,
    html_text: Option<String>,
    rtf_text: Option<String>,
    image_png: Option<String>,
    image_original_bytes: Option<String>,
    image_original_mime: Option<String>,
    image_preview_png: Option<String>,
    image_width: Option<u32>,
    image_height: Option<u32>,
    source_app: Option<String>,
    source_icon_data_url: Option<String>,
    hash: String,
    pinned: bool,
    favorite: bool,
    tag_colors: Vec<String>,
    updated_at: String,
    sync_updated_at: String,
    sync_device_id: String,
}

#[derive(Debug, Clone)]
struct WebdavClient {
    client: Client,
    settings: WebdavSyncSettings,
    password: String,
}

#[derive(Debug)]
struct RemoteManifest {
    value: WebdavManifest,
    etag: Option<String>,
}

impl Default for WebdavManifest {
    fn default() -> Self {
        Self {
            version: MANIFEST_VERSION,
            items: HashMap::new(),
            deleted: HashMap::new(),
        }
    }
}

impl WebdavClient {
    fn new(settings: WebdavSyncSettings, password: String) -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            settings,
            password,
        })
    }

    async fn ensure_collections(&self) -> Result<()> {
        let mut segments = Vec::new();
        if !self.settings.remote_dir.trim().is_empty() {
            for segment in self.settings.remote_dir.split('/') {
                if !segment.trim().is_empty() {
                    segments.push(segment.to_string());
                    self.ensure_collection(&segments).await?;
                }
            }
        }

        segments.push(SYNC_ROOT_DIR.into());
        self.ensure_collection(&segments).await?;
        segments.push(ITEMS_DIR.into());
        self.ensure_collection(&segments).await?;
        Ok(())
    }

    async fn test_connection(&self) -> Result<()> {
        self.ensure_collections().await?;
        let response = self
            .client
            .request(
                propfind_method()?,
                self.url_for_segments(&self.sync_segments([]))?,
            )
            .basic_auth(&self.settings.username, Some(&self.password))
            .header("Depth", "0")
            .send()
            .await?;
        if response.status() == StatusCode::GONE {
            anyhow::bail!("webdav_endpoint_gone");
        }
        if !response.status().is_success() && response.status() != StatusCode::MULTI_STATUS {
            anyhow::bail!("webdav_connection_failed: {}", response.status());
        }
        Ok(())
    }

    async fn fetch_manifest(&self) -> Result<RemoteManifest> {
        let manifest_url = self.manifest_url()?;
        let response = send_with_retry(self.is_nutstore_webdav(), || {
            self.client
                .get(manifest_url.clone())
                .basic_auth(&self.settings.username, Some(&self.password))
                .send()
        })
        .await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(RemoteManifest {
                value: WebdavManifest::default(),
                etag: None,
            });
        }

        let response = ensure_success(response, "webdav_manifest_fetch_failed")?;
        let etag = response
            .headers()
            .get(ETAG)
            .and_then(|value| value.to_str().ok())
            .map(ToString::to_string);
        let value = response.json::<WebdavManifest>().await?;
        Ok(RemoteManifest { value, etag })
    }

    async fn put_manifest(&self, manifest: &WebdavManifest, etag: Option<&str>) -> Result<bool> {
        let manifest_url = self.manifest_url()?;
        let response = send_with_retry(self.is_nutstore_webdav(), || {
            let mut request = self
                .client
                .put(manifest_url.clone())
                .basic_auth(&self.settings.username, Some(&self.password))
                .json(manifest);
            if let Some(etag) = etag {
                request = request.header("If-Match", etag);
            }
            request.send()
        })
        .await?;
        if response.status() == StatusCode::PRECONDITION_FAILED {
            return Ok(false);
        }
        ensure_success(response, "webdav_manifest_put_failed")?;
        Ok(true)
    }

    async fn get_item(&self, id: &str) -> Result<StoredClipboardItem> {
        let item_url = self.item_url(id)?;
        let response = send_with_retry(self.is_nutstore_webdav(), || {
            self.client
                .get(item_url.clone())
                .basic_auth(&self.settings.username, Some(&self.password))
                .send()
        })
        .await?;
        let response = ensure_success(response, "webdav_item_fetch_failed")?;
        let item = response.json::<SyncedClipboardItem>().await?;
        item.try_into()
    }

    async fn put_item(&self, item: &StoredClipboardItem) -> Result<()> {
        let item_url = self.item_url(&item.id)?;
        let synced_item = SyncedClipboardItem::from(item);
        let response = send_with_retry(self.is_nutstore_webdav(), || {
            self.client
                .put(item_url.clone())
                .basic_auth(&self.settings.username, Some(&self.password))
                .json(&synced_item)
                .send()
        })
        .await?;
        ensure_success(response, "webdav_item_put_failed")?;
        Ok(())
    }

    async fn delete_item(&self, id: &str) -> Result<()> {
        let item_url = self.item_url(id)?;
        let response = send_with_retry(self.is_nutstore_webdav(), || {
            self.client
                .delete(item_url.clone())
                .basic_auth(&self.settings.username, Some(&self.password))
                .send()
        })
        .await?;
        if matches!(response.status(), StatusCode::NOT_FOUND | StatusCode::GONE) {
            return Ok(());
        }
        ensure_success(response, "webdav_item_delete_failed")?;
        Ok(())
    }

    async fn ensure_collection(&self, segments: &[String]) -> Result<()> {
        let url = self.collection_url_for_segments(segments)?;
        let response = self
            .client
            .request(propfind_method()?, url.clone())
            .basic_auth(&self.settings.username, Some(&self.password))
            .header("Depth", "0")
            .send()
            .await?;
        if response.status() == StatusCode::UNAUTHORIZED {
            anyhow::bail!("webdav_mkcol_failed: {}", response.status());
        }
        if response.status().is_success() || response.status() == StatusCode::MULTI_STATUS {
            return Ok(());
        }

        let response = self
            .client
            .request(mkcol_method()?, url)
            .basic_auth(&self.settings.username, Some(&self.password))
            .send()
            .await?;
        if response.status() == StatusCode::UNAUTHORIZED {
            anyhow::bail!("webdav_mkcol_failed: {}", response.status());
        }
        if response.status().is_success()
            || response.status() == StatusCode::METHOD_NOT_ALLOWED
            || response.status() == StatusCode::CONFLICT
        {
            return Ok(());
        }
        eprintln!(
            "webdav collection check skipped after server returned {}",
            response.status()
        );
        Ok(())
    }

    fn base_url(&self) -> Result<Url> {
        Url::parse(&self.settings.server_url)
            .with_context(|| "webdav_invalid_server_url".to_string())
    }

    fn manifest_url(&self) -> Result<Url> {
        self.url_for_segments(&self.sync_segments([MANIFEST_FILE]))
    }

    fn item_url(&self, id: &str) -> Result<Url> {
        self.url_for_segments(&self.sync_segments([ITEMS_DIR, &format!("{id}.json")]))
    }

    fn sync_segments<'a, I>(&self, extra: I) -> Vec<String>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut segments = Vec::new();
        for segment in self.settings.remote_dir.split('/') {
            if !segment.trim().is_empty() {
                segments.push(segment.to_string());
            }
        }
        segments.push(SYNC_ROOT_DIR.into());
        segments.extend(extra.into_iter().map(ToString::to_string));
        segments
    }

    fn url_for_segments(&self, segments: &[String]) -> Result<Url> {
        let mut url = self.base_url()?;
        {
            let mut path_segments = url
                .path_segments_mut()
                .map_err(|_| anyhow::anyhow!("webdav_invalid_server_url"))?;
            for segment in segments {
                path_segments.push(segment);
            }
        }
        Ok(url)
    }

    fn collection_url_for_segments(&self, segments: &[String]) -> Result<Url> {
        let mut url = self.url_for_segments(segments)?;
        {
            let mut path_segments = url
                .path_segments_mut()
                .map_err(|_| anyhow::anyhow!("webdav_invalid_server_url"))?;
            path_segments.push("");
        }
        Ok(url)
    }

    fn is_nutstore_webdav(&self) -> bool {
        self.base_url()
            .ok()
            .and_then(|url| url.host_str().map(|host| host.to_ascii_lowercase()))
            .is_some_and(|host| host.ends_with("dav.jianguoyun.com"))
    }
}

impl From<&StoredClipboardItem> for SyncedClipboardItem {
    fn from(item: &StoredClipboardItem) -> Self {
        Self {
            id: item.id.clone(),
            kind: item.kind.clone(),
            created_at: item.created_at.clone(),
            pinned_at: item.pinned_at.clone(),
            preview: item.preview.clone(),
            full_text: item.full_text.clone(),
            html_text: item.html_text.clone(),
            rtf_text: item.rtf_text.clone(),
            image_png: encode_optional(item.image_png.as_deref()),
            image_original_bytes: encode_optional(item.image_original_bytes.as_deref()),
            image_original_mime: item.image_original_mime.clone(),
            image_preview_png: encode_optional(item.image_preview_png.as_deref()),
            image_width: item.image_width,
            image_height: item.image_height,
            source_app: item.source_app.clone(),
            source_icon_data_url: item.source_icon_data_url.clone(),
            hash: item.hash.clone(),
            pinned: item.pinned,
            favorite: item.favorite,
            tag_colors: item.tag_colors.clone(),
            updated_at: item.updated_at.clone(),
            sync_updated_at: item.sync_updated_at.clone(),
            sync_device_id: item.sync_device_id.clone(),
        }
    }
}

impl TryFrom<SyncedClipboardItem> for StoredClipboardItem {
    type Error = anyhow::Error;

    fn try_from(item: SyncedClipboardItem) -> Result<Self> {
        Ok(Self {
            id: item.id,
            kind: item.kind,
            created_at: item.created_at,
            pinned_at: item.pinned_at,
            preview: item.preview,
            full_text: item.full_text,
            html_text: item.html_text,
            rtf_text: item.rtf_text,
            image_png: decode_optional(item.image_png.as_deref())?,
            image_original_bytes: decode_optional(item.image_original_bytes.as_deref())?,
            image_original_mime: item.image_original_mime,
            image_preview_png: decode_optional(item.image_preview_png.as_deref())?,
            image_width: item.image_width,
            image_height: item.image_height,
            source_app: item.source_app,
            source_icon_data_url: item.source_icon_data_url,
            hash: item.hash,
            pinned: item.pinned,
            favorite: item.favorite,
            tag_colors: item.tag_colors,
            copy_count: 0,
            updated_at: item.updated_at,
            sync_updated_at: item.sync_updated_at,
            sync_device_id: item.sync_device_id,
        })
    }
}

pub(crate) fn schedule_auto_sync(app: AppHandle, shared: Arc<SharedState>) {
    if !shared.settings.lock().unwrap().webdav_sync.auto_sync
        || !shared.settings.lock().unwrap().webdav_sync.enabled
    {
        return;
    }

    if shared.webdav_sync_running.load(Ordering::Relaxed) {
        shared.webdav_sync_pending.store(true, Ordering::Relaxed);
        return;
    }

    tauri::async_runtime::spawn(async move {
        let _ = sync_once(app, shared, true).await;
    });
}

pub(crate) fn current_webdav_sync_state(
    shared: &Arc<SharedState>,
) -> Result<WebdavSyncStatusDto, AppError> {
    Ok(shared.webdav_sync_status.lock().unwrap().clone())
}

// 保存 WebDAV 密码到系统凭据存储，settings.json 不保存密码明文。
pub(crate) fn update_webdav_credential(
    shared: &Arc<SharedState>,
    payload: WebdavCredentialPayload,
) -> Result<(), AppError> {
    let settings = shared.settings.lock().unwrap().webdav_sync.clone();
    for entry in credential_entries(&settings).map_err(AppError::from)? {
        entry
            .set_password(&payload.password)
            .map_err(|error| AppError::Message(error.to_string()))?;
    }
    Ok(())
}

// 清除系统凭据存储中的 WebDAV 密码。
pub(crate) fn clear_webdav_credential(shared: &Arc<SharedState>) -> Result<(), AppError> {
    let settings = shared.settings.lock().unwrap().webdav_sync.clone();
    for entry in credential_entries(&settings).map_err(AppError::from)? {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(error) => return Err(AppError::Message(error.to_string())),
        }
    }
    Ok(())
}

// 测试当前 WebDAV 配置和系统凭据是否可访问。
pub(crate) async fn test_webdav_sync(
    shared: Arc<SharedState>,
) -> Result<WebdavSyncStatusDto, AppError> {
    let settings = shared.settings.lock().unwrap().webdav_sync.clone();
    let client = build_client(settings).map_err(AppError::from)?;
    client.test_connection().await.map_err(AppError::from)?;
    Ok(WebdavSyncStatusDto {
        status: "connected".into(),
        last_sync_at: shared
            .history_store
            .lock()
            .unwrap()
            .last_sync_at()
            .map_err(AppError::from)?,
        error: None,
        changed_count: 0,
    })
}

// 立即执行一次 WebDAV 双向同步。
pub(crate) async fn sync_webdav_now(
    app: AppHandle,
    shared: Arc<SharedState>,
) -> Result<WebdavSyncStatusDto, AppError> {
    sync_once(app, shared, false).await.map_err(AppError::from)
}

pub(crate) async fn sync_once(
    app: AppHandle,
    shared: Arc<SharedState>,
    automatic: bool,
) -> Result<WebdavSyncStatusDto> {
    if shared.webdav_sync_running.swap(true, Ordering::Relaxed) {
        shared.webdav_sync_pending.store(true, Ordering::Relaxed);
        return Ok(shared.webdav_sync_status.lock().unwrap().clone());
    }

    let mut result = sync_once_inner(app.clone(), shared.clone()).await;

    if automatic {
        while shared.webdav_sync_pending.swap(false, Ordering::Relaxed) {
            result = sync_once_inner(app.clone(), shared.clone()).await;
        }
    }

    shared.webdav_sync_running.store(false, Ordering::Relaxed);
    result
}

async fn sync_once_inner(app: AppHandle, shared: Arc<SharedState>) -> Result<WebdavSyncStatusDto> {
    emit_status(
        &app,
        &shared,
        WebdavSyncStatusDto {
            status: "syncing".into(),
            last_sync_at: shared.history_store.lock().unwrap().last_sync_at()?,
            error: None,
            changed_count: 0,
        },
    );

    let settings = shared.settings.lock().unwrap().webdav_sync.clone();
    if !settings.enabled {
        let status =
            WebdavSyncStatusDto::idle(shared.history_store.lock().unwrap().last_sync_at()?);
        emit_status(&app, &shared, status.clone());
        return Ok(status);
    }

    let client = build_client(settings)?;
    let result = perform_sync(&client, &shared).await;
    match result {
        Ok(changed_count) => {
            let last_sync_at = Utc::now().to_rfc3339();
            shared
                .history_store
                .lock()
                .unwrap()
                .set_last_sync_at(&last_sync_at)?;
            if changed_count > 0 {
                let _ = app.emit(crate::models::HISTORY_UPDATED_EVENT, ());
            }
            let status = WebdavSyncStatusDto {
                status: "idle".into(),
                last_sync_at: Some(last_sync_at),
                error: None,
                changed_count,
            };
            emit_status(&app, &shared, status.clone());
            Ok(status)
        }
        Err(error) => {
            let status = WebdavSyncStatusDto {
                status: "error".into(),
                last_sync_at: shared.history_store.lock().unwrap().last_sync_at()?,
                error: Some(error.to_string()),
                changed_count: 0,
            };
            emit_status(&app, &shared, status.clone());
            Err(error)
        }
    }
}

async fn perform_sync(client: &WebdavClient, shared: &Arc<SharedState>) -> Result<usize> {
    client.ensure_collections().await?;

    for _ in 0..2 {
        let remote = client.fetch_manifest().await?;
        let mut manifest = remote.value;
        let mut changed_count = apply_remote_changes(client, shared, &manifest).await?;
        changed_count += upload_local_changes(client, shared, &mut manifest).await?;

        if client
            .put_manifest(&manifest, remote.etag.as_deref())
            .await?
        {
            return Ok(changed_count);
        }
    }

    anyhow::bail!("webdav_manifest_conflict")
}

async fn apply_remote_changes(
    client: &WebdavClient,
    shared: &Arc<SharedState>,
    manifest: &WebdavManifest,
) -> Result<usize> {
    let mut changed_count = 0;
    for (id, entry) in &manifest.deleted {
        let deletion = DeletedClipboardItem {
            id: id.clone(),
            deleted_at: entry.sync_updated_at.clone(),
            sync_updated_at: entry.sync_updated_at.clone(),
            sync_device_id: entry.sync_device_id.clone(),
        };
        if shared
            .history_store
            .lock()
            .unwrap()
            .apply_synced_deletion(&deletion)?
        {
            changed_count += 1;
        }
    }

    let local_items = shared.history_store.lock().unwrap().list_sync_items()?;
    let local_items = local_items
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect::<HashMap<_, _>>();
    let local_deletions = shared
        .history_store
        .lock()
        .unwrap()
        .list_deleted_sync_items()?;
    let local_deletions = local_deletions
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect::<HashMap<_, _>>();

    for (id, entry) in &manifest.items {
        if local_deletions.get(id).is_some_and(|deletion| {
            !is_remote_newer(
                &entry.sync_updated_at,
                &entry.sync_device_id,
                &deletion.sync_updated_at,
                &deletion.sync_device_id,
            )
        }) {
            continue;
        }
        if local_items.get(id).is_some_and(|item| {
            !is_remote_newer(
                &entry.sync_updated_at,
                &entry.sync_device_id,
                &item.sync_updated_at,
                &item.sync_device_id,
            )
        }) {
            continue;
        }

        let item = client.get_item(id).await?;
        if shared
            .history_store
            .lock()
            .unwrap()
            .upsert_synced_item(&item)?
        {
            changed_count += 1;
        }
    }
    Ok(changed_count)
}

async fn upload_local_changes(
    client: &WebdavClient,
    shared: &Arc<SharedState>,
    manifest: &mut WebdavManifest,
) -> Result<usize> {
    let mut changed_count = 0;
    let local_items = shared.history_store.lock().unwrap().list_sync_items()?;
    for item in local_items {
        let should_upload = manifest.items.get(&item.id).map_or(true, |entry| {
            is_remote_newer(
                &item.sync_updated_at,
                &item.sync_device_id,
                &entry.sync_updated_at,
                &entry.sync_device_id,
            )
        });
        if should_upload {
            client.put_item(&item).await?;
            manifest.deleted.remove(&item.id);
            manifest.items.insert(
                item.id.clone(),
                WebdavManifestEntry {
                    sync_updated_at: item.sync_updated_at.clone(),
                    sync_device_id: item.sync_device_id.clone(),
                },
            );
            changed_count += 1;
        }
    }

    let local_deletions = shared
        .history_store
        .lock()
        .unwrap()
        .list_deleted_sync_items()?;
    for deletion in local_deletions {
        let should_upload = manifest.deleted.get(&deletion.id).map_or(true, |entry| {
            is_remote_newer(
                &deletion.sync_updated_at,
                &deletion.sync_device_id,
                &entry.sync_updated_at,
                &entry.sync_device_id,
            )
        });
        if should_upload {
            client.delete_item(&deletion.id).await?;
            manifest.items.remove(&deletion.id);
            manifest.deleted.insert(
                deletion.id.clone(),
                WebdavManifestEntry {
                    sync_updated_at: deletion.sync_updated_at.clone(),
                    sync_device_id: deletion.sync_device_id.clone(),
                },
            );
            changed_count += 1;
        }
    }

    manifest.version = MANIFEST_VERSION;
    Ok(changed_count)
}

fn build_client(settings: WebdavSyncSettings) -> Result<WebdavClient> {
    let settings = settings.normalized();
    if settings.server_url.is_empty() || settings.username.is_empty() {
        anyhow::bail!("webdav_settings_incomplete");
    }
    let password = read_credential(&settings)?;
    WebdavClient::new(settings, password)
}

fn credential_entries(settings: &WebdavSyncSettings) -> Result<Vec<keyring::Entry>> {
    let settings = settings.clone().normalized();
    let mut keys = Vec::new();
    if !settings.server_url.is_empty() && !settings.username.is_empty() {
        keys.push(format!("{}|{}", settings.server_url, settings.username));
    }
    if !settings.username.is_empty() {
        keys.push(format!("user|{}", settings.username));
    }
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .map(|key| keyring::Entry::new(CREDENTIAL_SERVICE, &key).map_err(Into::into))
        .collect()
}

fn legacy_credential_entry(settings: &WebdavSyncSettings) -> Result<keyring::Entry> {
    let key = format!("{}|{}", settings.server_url, settings.username);
    keyring::Entry::new(CREDENTIAL_SERVICE, &key).map_err(Into::into)
}

fn read_credential(settings: &WebdavSyncSettings) -> Result<String> {
    let mut last_error = None;
    for entry in credential_entries(settings)? {
        match entry.get_password() {
            Ok(password) => return Ok(password),
            Err(keyring::Error::NoEntry) => {}
            Err(error) => last_error = Some(error.to_string()),
        }
    }

    match legacy_credential_entry(settings)?.get_password() {
        Ok(password) => Ok(password),
        Err(keyring::Error::NoEntry) => {
            if let Some(error) = last_error {
                anyhow::bail!("webdav_credential_missing: {error}");
            }
            anyhow::bail!("webdav_credential_missing")
        }
        Err(error) => anyhow::bail!("webdav_credential_missing: {error}"),
    }
}

fn emit_status(app: &AppHandle, shared: &Arc<SharedState>, status: WebdavSyncStatusDto) {
    *shared.webdav_sync_status.lock().unwrap() = status.clone();
    let _ = app.emit(WEBDAV_SYNC_STATUS_EVENT, status);
}

fn propfind_method() -> Result<Method> {
    Method::from_bytes(b"PROPFIND").map_err(Into::into)
}

fn mkcol_method() -> Result<Method> {
    Method::from_bytes(b"MKCOL").map_err(Into::into)
}

fn ensure_success(response: reqwest::Response, error_code: &str) -> Result<reqwest::Response> {
    if response.status() == StatusCode::GONE {
        anyhow::bail!("webdav_endpoint_gone");
    }
    if !response.status().is_success() {
        anyhow::bail!("{error_code}: {}", response.status());
    }
    Ok(response)
}

async fn send_with_retry<F, Fut>(
    stop_on_nutstore_service_unavailable: bool,
    mut build_request: F,
) -> Result<reqwest::Response>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = std::result::Result<reqwest::Response, reqwest::Error>>,
{
    let mut last_error = None;
    for attempt in 0..WEBDAV_RETRY_ATTEMPTS {
        match build_request().await {
            Ok(response)
                if stop_on_nutstore_service_unavailable
                    && response.status() == StatusCode::SERVICE_UNAVAILABLE =>
            {
                anyhow::bail!("webdav_nutstore_request_limit: 503 Service Unavailable")
            }
            Ok(response)
                if is_transient_status(response.status())
                    && attempt + 1 < WEBDAV_RETRY_ATTEMPTS =>
            {
                let wait = retry_delay(attempt);
                eprintln!(
                    "webdav transient status {}, retrying ({}/{}) after {}ms",
                    response.status(),
                    attempt + 1,
                    WEBDAV_RETRY_ATTEMPTS,
                    wait.as_millis()
                );
                std::thread::sleep(wait);
                continue;
            }
            Ok(response) => return Ok(response),
            Err(error) if attempt + 1 < WEBDAV_RETRY_ATTEMPTS => {
                last_error = Some(error.to_string());
                let wait = retry_delay(attempt);
                eprintln!(
                    "webdav transient request error, retrying ({}/{}) after {}ms: {}",
                    attempt + 1,
                    WEBDAV_RETRY_ATTEMPTS,
                    wait.as_millis(),
                    last_error.as_deref().unwrap_or_default()
                );
                std::thread::sleep(wait);
            }
            Err(error) => return Err(error.into()),
        }
    }

    anyhow::bail!(
        "webdav_retry_exhausted: {}",
        last_error.unwrap_or_else(|| "transient status".into())
    )
}

fn retry_delay(attempt: usize) -> Duration {
    const DELAYS_MS: [u64; 2] = [1200, 2500];
    Duration::from_millis(DELAYS_MS.get(attempt).copied().unwrap_or(2500))
}

fn is_transient_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS
            | StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

fn encode_optional(bytes: Option<&[u8]>) -> Option<String> {
    bytes.map(|bytes| BASE64.encode(bytes))
}

fn decode_optional(value: Option<&str>) -> Result<Option<Vec<u8>>> {
    value
        .filter(|value| !value.is_empty())
        .map(|value| BASE64.decode(value).map_err(Into::into))
        .transpose()
}

fn is_remote_newer(
    remote_updated_at: &str,
    remote_device_id: &str,
    local_updated_at: &str,
    local_device_id: &str,
) -> bool {
    match remote_updated_at.cmp(local_updated_at) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Equal => remote_device_id > local_device_id,
        std::cmp::Ordering::Less => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        decode_optional, encode_optional, is_remote_newer, retry_delay, WebdavClient,
        WEBDAV_RETRY_ATTEMPTS,
    };
    use crate::models::WebdavSyncSettings;

    #[test]
    fn encodes_binary_fields_as_base64() {
        let encoded = encode_optional(Some(b"hello")).expect("encoded");
        assert_eq!(
            decode_optional(Some(&encoded)).expect("decoded"),
            Some(b"hello".to_vec())
        );
    }

    #[test]
    fn uses_timestamp_then_device_id_for_conflicts() {
        assert!(is_remote_newer("2026-01-02", "a", "2026-01-01", "z"));
        assert!(is_remote_newer("2026-01-02", "z", "2026-01-02", "a"));
        assert!(!is_remote_newer("2026-01-01", "z", "2026-01-02", "a"));
    }

    #[test]
    fn limits_webdav_retries_to_three_attempts() {
        assert_eq!(WEBDAV_RETRY_ATTEMPTS, 3);
        assert_eq!(retry_delay(0).as_millis(), 1200);
        assert_eq!(retry_delay(1).as_millis(), 2500);
        assert_eq!(retry_delay(2).as_millis(), 2500);
    }

    #[test]
    fn detects_nutstore_webdav_endpoint() {
        let settings = WebdavSyncSettings {
            server_url: "https://dav.jianguoyun.com/dav".into(),
            username: "user".into(),
            ..Default::default()
        };
        let client = WebdavClient::new(settings, "password".into()).expect("client");
        assert!(client.is_nutstore_webdav());

        let settings = WebdavSyncSettings {
            server_url: "https://example.com/dav".into(),
            username: "user".into(),
            ..Default::default()
        };
        let client = WebdavClient::new(settings, "password".into()).expect("client");
        assert!(!client.is_nutstore_webdav());
    }
}
