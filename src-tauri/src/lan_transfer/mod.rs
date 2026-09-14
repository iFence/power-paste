//! 基于内嵌 LocalSend 协议核心库实现的局域网互传。
//!
//! 常驻监听 53317 端口并参与组播发现，因而安装了 power-paste 就能与官方 LocalSend
//! 客户端互相发现与传输；浏览器扫码页复用同一套协议端点。

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, Weak,
    },
    time::Duration,
};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use localsend::{
    discovery::{
        DeviceIdentity, DiscoveryConfig, DiscoveryEvent, DiscoveryHandle, DEFAULT_DISCOVERY_TIMEOUT,
    },
    http::server::{
        start_with_port,
        v2::ServerEventV2,
        web::{WebConfig, WebSendEvent},
        ServerConfigV2, ServerHandle,
    },
    model::discovery::ProtocolType,
    multicast::{DEFAULT_MULTICAST_GROUP, DEFAULT_MULTICAST_GROUP_V6, DEFAULT_PORT},
    util::interface::InterfaceFilter,
};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::models::{AppError, AppSettings, LanTrustedDevice, SharedState};

mod identity;
mod receive;
mod scan;
mod selection;
mod send;
mod text_package;
mod util;
mod web_link;

use identity::LanIdentity;
pub(crate) use scan::{list_subnets, LanScanDto, LanSubnetsDto};
use scan::{ScanScope, ScanState};
pub(crate) use selection::{inspect_selection, read_clipboard_selection, LanSelectionItemDto};
pub(crate) use text_package::{is_text_package, TEXT_RESTORE_MAX_BYTES};
pub(crate) use util::validate_download_dir;

// 前端状态事件：任意变化都会推送完整状态，前端直接替换即可。
pub(crate) const LAN_STATE_EVENT: &str = "lan-transfer-state";

// 等待用户确认接收的超时时间，超时按拒绝处理，避免对端一直挂起。
pub(crate) const INCOMING_TIMEOUT: Duration = Duration::from_secs(45);

// 传输记录与已接收文件在内存中的上限，避免长时间运行后无限增长（磁盘文件不删除）。
const MAX_TRANSFER_ENTRIES: usize = 50;
const MAX_RECEIVED_ENTRIES: usize = 200;
// 已知对端上限：会话列表只用于最近联系过 / 见过的设备。
const MAX_PEER_ENTRIES: usize = 100;

// 服务自愈：监听器或组播永久失败后在 60 秒窗口内最多重建 3 次。
const RECOVERY_WINDOW_MS: u64 = 60_000;
const MAX_RECOVERY_ATTEMPTS: u32 = 3;

// 客户端请求的用途，决定是否设置总超时。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClientPurpose {
    // 传输链路：对端可能需要人工确认，文件本身也可能很大，不能设置总超时；
    // 中断交给用户取消或底层连接错误处理。
    Transfer,
    // 设备发现：局域网设备要么很快应答，要么根本不存在，必须快速结束。
    Discovery,
}

// 按用途返回请求总超时；None 表示不设置总超时。
pub(crate) fn client_timeout_for(purpose: ClientPurpose) -> Option<Duration> {
    match purpose {
        ClientPurpose::Transfer => None,
        ClientPurpose::Discovery => Some(DEFAULT_DISCOVERY_TIMEOUT),
    }
}

// 扫码页模式对应的对外广播能力：协议与下载 API 状态必须与服务端实际监听方式一致。
pub(crate) fn advertised_device(mode: &str) -> (ProtocolType, bool) {
    match mode {
        "share" => (ProtocolType::Http, true),
        "receive" => (ProtocolType::Http, false),
        _ => (ProtocolType::Https, false),
    }
}

/// 前端可见的局域网互传状态。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanStateDto {
    pub(crate) status: String,
    pub(crate) error: Option<String>,
    // 服务级错误的稳定错误码，前端据此本地化展示；error 保留原始细节。
    pub(crate) error_code: Option<String>,
    pub(crate) warning: Option<String>,
    // 设置页里的"启用局域网互传"开关，关闭时前端不自动启动服务。
    pub(crate) enabled: bool,
    pub(crate) alias: String,
    pub(crate) port: u16,
    pub(crate) fingerprint: Option<String>,
    pub(crate) devices: Vec<LanDeviceDto>,
    // 已知对端（含当前离线的历史设备），会话列表按它渲染。
    pub(crate) peers: Vec<LanDeviceDto>,
    pub(crate) transfers: Vec<LanTransferDto>,
    pub(crate) incoming: Option<LanIncomingRequestDto>,
    pub(crate) received_files: Vec<LanReceivedFileDto>,
    pub(crate) web_mode: String,
    pub(crate) web_url: Option<String>,
    pub(crate) web_qr_svg: Option<String>,
    // 刷新/网段扫描进度；未扫描过时为 None。
    pub(crate) scan: Option<LanScanDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanDeviceDto {
    pub(crate) fingerprint: String,
    pub(crate) alias: String,
    pub(crate) device_model: Option<String>,
    pub(crate) device_type: Option<String>,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) protocol: String,
    pub(crate) trusted: bool,
    // 是否存在于当前离线发现列表：会话列表据此区分在线与离线。
    pub(crate) online: bool,
    pub(crate) last_seen_ms: u64,
}

// 一次传输涉及的文件元数据，供会话气泡展示文件名与体积。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanFileDto {
    pub(crate) file_name: String,
    // MIME 类型：会话气泡据此决定展示图片预览还是文件名。
    pub(crate) mime_type: String,
    pub(crate) size: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanTransferDto {
    pub(crate) id: String,
    pub(crate) direction: String,
    pub(crate) peer_fingerprint: String,
    pub(crate) peer_alias: String,
    pub(crate) status: String,
    // text / files：文本消息与文件传输在会话里渲染成不同气泡。
    pub(crate) kind: String,
    pub(crate) label: String,
    pub(crate) text: Option<String>,
    pub(crate) files: Vec<LanFileDto>,
    pub(crate) total_bytes: u64,
    pub(crate) done_bytes: u64,
    pub(crate) error: Option<String>,
    pub(crate) created_at_ms: u64,
    // 仅发送方向且在内存里保留了原始选择项时才可重新发送。
    pub(crate) resendable: bool,
    // 是否持有图片预览（通过 read_lan_transfer_preview 按需取回）。
    pub(crate) has_preview: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanIncomingRequestDto {
    pub(crate) request_id: String,
    pub(crate) alias: String,
    pub(crate) device_model: Option<String>,
    pub(crate) ip: String,
    pub(crate) fingerprint: String,
    pub(crate) trusted: bool,
    pub(crate) files: Vec<LanFileDto>,
    pub(crate) total_bytes: u64,
    pub(crate) text_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanReceivedFileDto {
    pub(crate) id: String,
    pub(crate) file_name: String,
    pub(crate) path: String,
    pub(crate) size: u64,
    pub(crate) from_alias: String,
    pub(crate) from_fingerprint: String,
    // 归属的传输记录，会话里据此把文件挂到对应气泡上。
    pub(crate) transfer_id: String,
    pub(crate) received_at_ms: u64,
}

// 用户对一次接收请求的决定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LanDecision {
    Accept,
    Decline,
    AcceptAndTrust,
}

struct DeviceEntry {
    fingerprint: String,
    alias: String,
    device_model: Option<String>,
    device_type: Option<String>,
    host: String,
    port: u16,
    protocol: ProtocolType,
    last_seen_ms: u64,
}

// 传输内容类型：文本消息不落文件，单独标记以便会话气泡还原正文。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferKind {
    Text,
    Files,
}

impl TransferKind {
    fn as_str(self) -> &'static str {
        match self {
            TransferKind::Text => "text",
            TransferKind::Files => "files",
        }
    }
}

struct TransferEntry {
    id: String,
    direction: &'static str,
    peer_fingerprint: String,
    peer_alias: String,
    status: &'static str,
    kind: TransferKind,
    label: String,
    // 文本消息正文；文件传输为 None。
    text: Option<String>,
    // 本次传输涉及的文件清单，供会话气泡展示。
    files: Vec<LanFileDto>,
    // 发送时的原始选择项，用于失败/取消后重新发送；接收方向为空。
    items: Vec<LanSelectionItemDto>,
    // 图片预览（缩略 PNG）：按需通过命令返回，不进入状态快照。
    preview_png: Option<Vec<u8>>,
    total_bytes: u64,
    done_bytes: Arc<AtomicU64>,
    error: Option<String>,
    created_at_ms: u64,
    // 发送时的会话 ID、对端地址与取消令牌，接收时为 None。
    session_id: Option<String>,
    peer_host: Option<String>,
    cancel: Option<CancellationToken>,
}

#[derive(Clone)]
struct IncomingEntry {
    request_id: String,
    session_id: String,
    alias: String,
    device_model: Option<String>,
    ip: String,
    fingerprint: String,
    // 待确认的文件名、体积与类型；确认弹窗据此展示文件清单。
    files: Vec<(String, u64, String)>,
    total_bytes: u64,
    text_message: Option<String>,
}

struct ReceivedEntry {
    id: String,
    file_name: String,
    path: PathBuf,
    size: u64,
    from_alias: String,
    from_fingerprint: String,
    transfer_id: String,
    received_at_ms: u64,
}

#[derive(Default)]
struct WebEntry {
    mode: String,
    url: Option<String>,
    qr_svg: Option<String>,
}

// 运行中的服务句柄；停止时先发信号再等待任务退出，保证端口可以重新绑定。
struct Service {
    server: Arc<ServerHandle>,
    server_stop: Option<oneshot::Sender<()>>,
    discovery: Arc<DiscoveryHandle>,
    discovery_stop: Option<oneshot::Sender<()>>,
    server_tx: mpsc::Sender<ServerEventV2>,
    web_tx: mpsc::Sender<WebSendEvent>,
}

struct Inner {
    status: String,
    error: Option<String>,
    error_code: Option<String>,
    warning: Option<String>,
    identity: Option<Arc<LanIdentity>>,
    devices: Vec<DeviceEntry>,
    // 会话列表数据源：记录见过的对端，服务停止后仍保留，因此离线设备也留有会话入口。
    peers: Vec<DeviceEntry>,
    // 对端要求的 PIN，按指纹缓存在内存中，避免同一设备每次发送都重新输入。
    device_pins: HashMap<String, String>,
    transfers: Vec<TransferEntry>,
    incoming: Option<IncomingEntry>,
    received: Vec<ReceivedEntry>,
    web: WebEntry,
    web_paths: HashMap<String, PathBuf>,
    sessions: HashMap<String, receive::ReceiveSession>,
    service: Option<Service>,
    pending: HashMap<String, oneshot::Sender<LanDecision>>,
    scan: Option<ScanState>,
}

impl Inner {
    fn new() -> Self {
        Self {
            status: "stopped".into(),
            error: None,
            error_code: None,
            warning: None,
            identity: None,
            devices: Vec::new(),
            peers: Vec::new(),
            device_pins: HashMap::new(),
            transfers: Vec::new(),
            incoming: None,
            received: Vec::new(),
            web: WebEntry {
                mode: "none".into(),
                url: None,
                qr_svg: None,
            },
            web_paths: HashMap::new(),
            sessions: HashMap::new(),
            service: None,
            pending: HashMap::new(),
            scan: None,
        }
    }

    // 记录或更新一个已确认的设备。
    #[allow(clippy::too_many_arguments)]
    fn upsert_device(
        &mut self,
        fingerprint: String,
        alias: String,
        device_model: Option<String>,
        device_type: Option<String>,
        host: String,
        port: u16,
        protocol: ProtocolType,
    ) {
        let now = util::now_ms();
        // 会话列表要保留历史对端，因此除在线列表外再维护一份去重的已知设备表。
        self.upsert_peer(
            fingerprint.clone(),
            alias.clone(),
            device_model.clone(),
            device_type.clone(),
            host.clone(),
            port,
            protocol,
            now,
        );
        if let Some(existing) = self
            .devices
            .iter_mut()
            .find(|device| device.fingerprint == fingerprint)
        {
            existing.alias = alias;
            existing.device_model = device_model;
            existing.device_type = device_type;
            existing.host = host;
            existing.port = port;
            existing.protocol = protocol;
            existing.last_seen_ms = now;
            return;
        }

        self.devices.push(DeviceEntry {
            fingerprint,
            alias,
            device_model,
            device_type,
            host,
            port,
            protocol,
            last_seen_ms: now,
        });
    }

    // 记录或更新一个已知对端；超出上限时丢弃最久未见的记录。
    #[allow(clippy::too_many_arguments)]
    fn upsert_peer(
        &mut self,
        fingerprint: String,
        alias: String,
        device_model: Option<String>,
        device_type: Option<String>,
        host: String,
        port: u16,
        protocol: ProtocolType,
        now: u64,
    ) {
        if let Some(existing) = self
            .peers
            .iter_mut()
            .find(|peer| peer.fingerprint == fingerprint)
        {
            existing.alias = alias;
            existing.device_model = device_model;
            existing.device_type = device_type;
            existing.host = host;
            existing.port = port;
            existing.protocol = protocol;
            existing.last_seen_ms = now;
            return;
        }

        self.peers.push(DeviceEntry {
            fingerprint,
            alias,
            device_model,
            device_type,
            host,
            port,
            protocol,
            last_seen_ms: now,
        });
        let overflow = self.peers.len().saturating_sub(MAX_PEER_ENTRIES);
        if overflow > 0 {
            // 先按最近可见时间排序，再丢弃最旧的部分，避免把活跃设备挤掉。
            self.peers.sort_by_key(|peer| peer.last_seen_ms);
            self.peers.drain(0..overflow);
        }
    }

    fn device(&self, fingerprint: &str) -> Option<&DeviceEntry> {
        self.devices
            .iter()
            .find(|device| device.fingerprint == fingerprint)
    }

    fn transfer(&mut self, id: &str) -> Option<&mut TransferEntry> {
        self.transfers.iter_mut().find(|entry| entry.id == id)
    }

    // 记录一次传输，只保留最近的若干条。
    fn push_transfer(&mut self, entry: TransferEntry) {
        self.transfers.push(entry);
        let overflow = self.transfers.len().saturating_sub(MAX_TRANSFER_ENTRIES);
        if overflow > 0 {
            self.transfers.drain(0..overflow);
        }
    }

    // 记录一个已接收文件，只保留最近的若干条。
    fn push_received(&mut self, entry: ReceivedEntry) {
        self.received.push(entry);
        let overflow = self.received.len().saturating_sub(MAX_RECEIVED_ENTRIES);
        if overflow > 0 {
            self.received.drain(0..overflow);
        }
    }

    fn finish_transfer(&mut self, id: &str, status: &'static str, error: Option<String>) {
        if let Some(entry) = self.transfer(id) {
            entry.status = status;
            entry.error = error;
            // 只有真正完成才拉满进度；取消或失败保留已传字节数。
            if status == "done" {
                let total = entry.total_bytes;
                entry.done_bytes.store(total, Ordering::Relaxed);
            }
        }
    }

    // 挂上图片预览，供会话气泡按需取回。
    fn set_transfer_preview(&mut self, id: &str, preview: Option<Vec<u8>>) {
        if let Some(preview) = preview {
            if let Some(entry) = self.transfer(id) {
                entry.preview_png = Some(preview);
            }
        }
    }

    // 整条传输只有一条文本时把它折叠成文本消息：气泡直接显示正文，不再显示 txt 文件名。
    fn mark_transfer_as_text(&mut self, id: &str, text: &str) {
        if let Some(entry) = self.transfer(id) {
            if entry.files.len() == 1 {
                entry.kind = TransferKind::Text;
                entry.text = Some(text.to_string());
                entry.files.clear();
                entry.label = "text".into();
            }
        }
    }
}

pub(crate) struct LanTransferHandle {
    inner: Mutex<Inner>,
    recovery: Mutex<RecoveryWindow>,
    // 设备确认次数：智能刷新据它判断局域网里是否已经有回应，决定要不要回退扫网段。
    confirmations: AtomicU64,
}

// 自愈窗口计数：避免监听器反复失败时无休止重建。
#[derive(Default)]
struct RecoveryWindow {
    started_ms: u64,
    attempts: u32,
    running: bool,
}

impl LanTransferHandle {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(Inner::new()),
            recovery: Mutex::new(RecoveryWindow::default()),
            confirmations: AtomicU64::new(0),
        }
    }

    // 记录或更新一个已确认的设备，并累加确认计数。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn upsert_device(
        &self,
        fingerprint: String,
        alias: String,
        device_model: Option<String>,
        device_type: Option<String>,
        host: String,
        port: u16,
        protocol: ProtocolType,
    ) {
        self.inner.lock().unwrap().upsert_device(
            fingerprint,
            alias,
            device_model,
            device_type,
            host,
            port,
            protocol,
        );
        self.confirmations.fetch_add(1, Ordering::Relaxed);
    }

    // 组装前端状态快照。
    pub(crate) fn state(&self, settings: &AppSettings) -> LanStateDto {
        let inner = self.inner.lock().unwrap();
        let identity = inner.identity.as_ref();
        let alias = identity
            .map(|value| value.alias.clone())
            .unwrap_or_else(|| resolve_alias(settings));
        let online: Vec<&str> = inner
            .devices
            .iter()
            .map(|device| device.fingerprint.as_str())
            .collect();

        LanStateDto {
            status: inner.status.clone(),
            error: inner.error.clone(),
            error_code: inner.error_code.clone(),
            warning: inner.warning.clone(),
            enabled: settings.lan_transfer_enabled,
            alias,
            port: identity.map(|value| value.port).unwrap_or(DEFAULT_PORT),
            fingerprint: identity.map(|value| value.fingerprint.clone()),
            devices: inner
                .devices
                .iter()
                .map(|device| device_dto(device, settings, true))
                .collect(),
            peers: inner
                .peers
                .iter()
                .map(|peer| {
                    let online = online.contains(&peer.fingerprint.as_str());
                    device_dto(peer, settings, online)
                })
                .collect(),
            transfers: inner
                .transfers
                .iter()
                .map(|entry| LanTransferDto {
                    id: entry.id.clone(),
                    direction: entry.direction.to_string(),
                    peer_fingerprint: entry.peer_fingerprint.clone(),
                    peer_alias: entry.peer_alias.clone(),
                    status: entry.status.to_string(),
                    kind: entry.kind.as_str().to_string(),
                    label: entry.label.clone(),
                    text: entry.text.clone(),
                    files: entry.files.clone(),
                    total_bytes: entry.total_bytes,
                    done_bytes: entry.done_bytes.load(Ordering::Relaxed),
                    error: entry.error.clone(),
                    created_at_ms: entry.created_at_ms,
                    resendable: entry.direction == "send" && !entry.items.is_empty(),
                    has_preview: entry.preview_png.is_some(),
                })
                .collect(),
            incoming: inner.incoming.as_ref().map(|entry| LanIncomingRequestDto {
                request_id: entry.request_id.clone(),
                alias: entry.alias.clone(),
                device_model: entry.device_model.clone(),
                ip: entry.ip.clone(),
                fingerprint: entry.fingerprint.clone(),
                trusted: settings
                    .lan_trusted_devices
                    .iter()
                    .any(|trusted| trusted.fingerprint == entry.fingerprint),
                files: entry
                    .files
                    .iter()
                    .map(|(file_name, size, mime_type)| LanFileDto {
                        file_name: file_name.clone(),
                        mime_type: mime_type.clone(),
                        size: *size,
                    })
                    .collect(),
                total_bytes: entry.total_bytes,
                text_message: entry.text_message.clone(),
            }),
            received_files: inner
                .received
                .iter()
                .rev()
                .map(|entry| LanReceivedFileDto {
                    id: entry.id.clone(),
                    file_name: entry.file_name.clone(),
                    path: entry.path.to_string_lossy().to_string(),
                    size: entry.size,
                    from_alias: entry.from_alias.clone(),
                    from_fingerprint: entry.from_fingerprint.clone(),
                    transfer_id: entry.transfer_id.clone(),
                    received_at_ms: entry.received_at_ms,
                })
                .collect(),
            web_mode: inner.web.mode.clone(),
            web_url: inner.web.url.clone(),
            web_qr_svg: inner.web.qr_svg.clone(),
            scan: inner.scan.as_ref().map(|value| value.to_dto()),
        }
    }

    // 启动局域网服务：监听端口、参与组播发现并推送状态事件。
    pub(crate) async fn start(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
    ) -> Result<LanStateDto, AppError> {
        let settings = shared.settings.lock().unwrap().clone();
        if self.inner.lock().unwrap().service.is_some() {
            return Ok(self.state(&settings));
        }

        let alias = resolve_alias(&settings);
        let dir = shared
            .paths
            .settings_path
            .parent()
            .ok_or_else(|| AppError::Message("settings parent missing".into()))?;
        let identity = Arc::new(
            LanIdentity::load_or_generate(dir, alias, DEFAULT_PORT).map_err(AppError::from)?,
        );

        {
            let mut inner = self.inner.lock().unwrap();
            inner.identity = Some(identity.clone());
            inner.error = None;
            inner.warning = None;
        }

        let service = match build_service(app, self, shared, identity.clone()).await {
            Ok(service) => service,
            Err(error) => {
                {
                    let mut inner = self.inner.lock().unwrap();
                    inner.status = "error".into();
                    inner.error_code = Some(error_code(&error.to_string()));
                    inner.error = Some(error.to_string());
                }
                emit_state(app, self, &settings);
                return Err(AppError::Message(error.to_string()));
            }
        };

        {
            let mut inner = self.inner.lock().unwrap();
            inner.status = "running".into();
            inner.error_code = None;
            inner.warning = service
                .discovery
                .multicast_error()
                .map(|_| "lan_multicast_unavailable".to_string());
            inner.service = Some(service);
        }

        emit_state(app, self, &settings);
        Ok(self.state(&settings))
    }

    // 停止局域网服务并释放端口。
    pub(crate) async fn stop(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
    ) -> Result<LanStateDto, AppError> {
        let service = self.inner.lock().unwrap().service.take();
        if let Some(service) = service {
            stop_service(service).await;
        }

        {
            let mut inner = self.inner.lock().unwrap();
            inner.status = "stopped".into();
            inner.error = None;
            inner.error_code = None;
            inner.warning = None;
            inner.devices.clear();
            inner.incoming = None;
            inner.pending.clear();
            inner.sessions.clear();
            inner.web_paths.clear();
            inner.web = WebEntry {
                mode: "none".into(),
                url: None,
                qr_svg: None,
            };
        }
        scan::clear(self);

        let settings = shared.settings.lock().unwrap().clone();
        emit_state(app, self, &settings);
        Ok(self.state(&settings))
    }

    // 主动刷新一次设备发现：先广播，再探测已知地址，必要时扫描本机全部网段。
    pub(crate) async fn refresh(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
    ) -> Result<LanStateDto, AppError> {
        scan::start(self, app, shared, ScanScope::Auto, self.port())?;
        let settings = shared.settings.lock().unwrap().clone();
        emit_state(app, self, &settings);
        Ok(self.state(&settings))
    }

    // 只扫描用户选定的网段，并记住这次选择。
    pub(crate) async fn scan_subnets(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
        subnets: Vec<String>,
        port: Option<u16>,
    ) -> Result<LanStateDto, AppError> {
        let subnets = scan::normalize_subnets(&subnets)?;
        if let Some(first) = subnets.first() {
            remember_scan_subnet(shared, &first.cidr)?;
        }

        let port = match port {
            Some(port) if port > 0 => port,
            _ => self.port(),
        };
        scan::start(self, app, shared, ScanScope::Subnets(subnets), port)?;

        let settings = shared.settings.lock().unwrap().clone();
        emit_state(app, self, &settings);
        Ok(self.state(&settings))
    }

    // 取消正在进行的扫描。
    pub(crate) fn cancel_scan(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
    ) -> LanStateDto {
        scan::cancel(self);
        let settings = shared.settings.lock().unwrap().clone();
        emit_state(app, self, &settings);
        self.state(&settings)
    }

    // 本机对外服务端口：身份尚未建立时回退到 LocalSend 默认端口。
    fn port(&self) -> u16 {
        self.inner
            .lock()
            .unwrap()
            .identity
            .as_ref()
            .map(|identity| identity.port)
            .unwrap_or(DEFAULT_PORT)
    }

    // 手动添加一台设备：直接向指定地址发送注册请求，用于组播不可用的网络。
    pub(crate) async fn discover_address(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
        host: String,
        port: u16,
    ) -> Result<LanStateDto, AppError> {
        let settings = shared.settings.lock().unwrap().clone();
        let discovery = {
            let inner = self.inner.lock().unwrap();
            inner
                .service
                .as_ref()
                .map(|service| service.discovery.clone())
        };
        let Some(discovery) = discovery else {
            return Err(AppError::Message("lan_transfer_not_running".into()));
        };

        let host = host.trim().to_string();
        if host.is_empty() {
            return Err(AppError::Message("lan_transfer_device_missing".into()));
        }
        let port = if port == 0 { DEFAULT_PORT } else { port };

        match discovery.discover(&host, port, ProtocolType::Https).await {
            Ok(Some(device)) => {
                if let Some(channel) = device.device.channel.http() {
                    self.upsert_device(
                        device.device.fingerprint.clone(),
                        device.device.alias.clone(),
                        device.device.device_model.clone(),
                        device.device.device_type.as_ref().map(device_type_label),
                        channel.host.clone(),
                        channel.port,
                        channel.protocol,
                    );
                }
            }
            Ok(None) => {
                return Err(AppError::Message("lan_transfer_device_missing".into()));
            }
            Err(error) => {
                return Err(AppError::Message(error.to_string()));
            }
        }

        emit_state(app, self, &settings);
        Ok(self.state(&settings))
    }

    // 向指定设备发送一个或多个文件。
    pub(crate) async fn send_files(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
        fingerprint: String,
        paths: Vec<String>,
        pin: Option<String>,
    ) -> Result<LanStateDto, AppError> {
        let settings = shared.settings.lock().unwrap().clone();
        let (identity, target) = {
            let inner = self.inner.lock().unwrap();
            let identity = inner
                .identity
                .clone()
                .ok_or_else(|| AppError::Message("lan_transfer_not_running".into()))?;
            let device = inner
                .device(&fingerprint)
                .ok_or_else(|| AppError::Message("lan_transfer_device_missing".into()))?;
            (
                identity,
                send::SendTarget {
                    alias: device.alias.clone(),
                    host: device.host.clone(),
                    port: device.port,
                    protocol: device.protocol,
                    fingerprint: device.fingerprint.clone(),
                    pin: resolve_pin(&inner, &fingerprint, pin),
                },
            )
        };

        let picked = send::collect_files(&paths)?;
        if picked.is_empty() {
            return Err(AppError::Message("lan_transfer_no_files".into()));
        }

        let transfer_id = uuid::Uuid::new_v4().to_string();
        let total_bytes = picked.iter().map(|picked| picked.file.size).sum::<u64>();
        let label = if picked.len() == 1 {
            picked[0].file.file_name.clone()
        } else {
            format!("{} files", picked.len())
        };
        // 保留原始选择项：失败或取消后可以按同一份内容重新发送。
        let items = picked
            .iter()
            .filter_map(|picked| match &picked.source {
                send::SendSource::Path(path) => Some(LanSelectionItemDto::File {
                    path: path.to_string_lossy().to_string(),
                    name: picked.file.file_name.clone(),
                    size: picked.file.size,
                    mime_type: picked.file.file_type.clone(),
                }),
                send::SendSource::Bytes(_) => None,
            })
            .collect::<Vec<_>>();
        let files = picked
            .iter()
            .map(|picked| LanFileDto {
                file_name: picked.file.file_name.clone(),
                mime_type: picked.file.file_type.clone(),
                size: picked.file.size,
            })
            .collect::<Vec<_>>();
        let preview_png = picked.iter().find_map(|picked| match &picked.source {
            send::SendSource::Path(path) if picked.file.file_type.starts_with("image/") => {
                util::image_preview_from_path(path)
            }
            _ => None,
        });

        {
            let mut inner = self.inner.lock().unwrap();
            inner.push_transfer(TransferEntry {
                id: transfer_id.clone(),
                direction: "send",
                peer_fingerprint: target.fingerprint.clone(),
                peer_alias: target.alias.clone(),
                status: "active",
                kind: TransferKind::Files,
                label,
                text: None,
                files,
                items,
                preview_png,
                total_bytes,
                done_bytes: Arc::new(AtomicU64::new(0)),
                error: None,
                created_at_ms: util::now_ms(),
                session_id: None,
                peer_host: Some(target.host.clone()),
                cancel: None,
            });
        }
        emit_state(app, self, &settings);

        let advertised_protocol = self.advertised_protocol();
        send::spawn_send(
            app.clone(),
            self.clone(),
            shared.clone(),
            identity,
            target,
            advertised_protocol,
            transfer_id,
            picked,
        );

        Ok(self.state(&settings))
    }

    // 向指定设备发送文本；使用协议原生文本消息，官方客户端会直接显示为消息。
    pub(crate) async fn send_text(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
        fingerprint: String,
        text: String,
        pin: Option<String>,
    ) -> Result<LanStateDto, AppError> {
        let text = text.trim_end().to_string();
        if text.is_empty() {
            return Err(AppError::Message("empty_payload".into()));
        }

        let settings = shared.settings.lock().unwrap().clone();
        let (identity, target) = {
            let inner = self.inner.lock().unwrap();
            let identity = inner
                .identity
                .clone()
                .ok_or_else(|| AppError::Message("lan_transfer_not_running".into()))?;
            let device = inner
                .device(&fingerprint)
                .ok_or_else(|| AppError::Message("lan_transfer_device_missing".into()))?;
            (
                identity,
                send::SendTarget {
                    alias: device.alias.clone(),
                    host: device.host.clone(),
                    port: device.port,
                    protocol: device.protocol,
                    fingerprint: device.fingerprint.clone(),
                    pin: resolve_pin(&inner, &fingerprint, pin),
                },
            )
        };

        let message = send::text_message(&text);
        let transfer_id = uuid::Uuid::new_v4().to_string();
        {
            let mut inner = self.inner.lock().unwrap();
            inner.push_transfer(TransferEntry {
                id: transfer_id.clone(),
                direction: "send",
                peer_fingerprint: target.fingerprint.clone(),
                peer_alias: target.alias.clone(),
                status: "active",
                kind: TransferKind::Text,
                label: "text".into(),
                text: Some(text.clone()),
                files: Vec::new(),
                items: vec![LanSelectionItemDto::text(text.clone())],
                preview_png: None,
                total_bytes: message.file.size,
                done_bytes: Arc::new(AtomicU64::new(text.len() as u64)),
                error: None,
                created_at_ms: util::now_ms(),
                session_id: None,
                peer_host: Some(target.host.clone()),
                cancel: None,
            });
        }
        emit_state(app, self, &settings);

        // 文本消息与文件走同一条发送链路：对端把它当消息时以 204 结束，
        // 当普通文件接受时会把正文真实上传，避免对端会话悬挂。
        let advertised_protocol = self.advertised_protocol();
        send::spawn_send(
            app.clone(),
            self.clone(),
            shared.clone(),
            identity,
            target,
            advertised_protocol,
            transfer_id,
            vec![message],
        );

        Ok(self.state(&settings))
    }

    // 回应一次待确认的接收请求。
    // 把文件、文件夹展开结果与文本消息合并为一次 LocalSend prepare-upload。
    pub(crate) async fn send_items(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
        fingerprint: String,
        items: Vec<LanSelectionItemDto>,
        pin: Option<String>,
    ) -> Result<LanStateDto, AppError> {
        let picked = selection::collect_items(&items)?;
        let settings = shared.settings.lock().unwrap().clone();
        let (identity, target) = {
            let inner = self.inner.lock().unwrap();
            let identity = inner
                .identity
                .clone()
                .ok_or_else(|| AppError::Message("lan_transfer_not_running".into()))?;
            let device = inner
                .device(&fingerprint)
                .ok_or_else(|| AppError::Message("lan_transfer_device_missing".into()))?;
            (
                identity,
                send::SendTarget {
                    alias: device.alias.clone(),
                    host: device.host.clone(),
                    port: device.port,
                    protocol: device.protocol,
                    fingerprint: device.fingerprint.clone(),
                    pin: resolve_pin(&inner, &fingerprint, pin),
                },
            )
        };

        let transfer_id = uuid::Uuid::new_v4().to_string();
        let total_bytes = picked.iter().map(|entry| entry.file.size).sum::<u64>();
        let label = if picked.len() == 1 {
            if matches!(items.as_slice(), [LanSelectionItemDto::Text { .. }]) {
                "text".into()
            } else {
                picked[0].file.file_name.clone()
            }
        } else {
            format!("{} items", picked.len())
        };
        // 文本与附件合并成同一次传输；协议原生文本只是承载方式，不算作文件。
        let text = match items.as_slice() {
            [LanSelectionItemDto::Text { text }] => Some(text.clone()),
            _ => None,
        };
        let files = items
            .iter()
            .zip(picked.iter())
            .filter_map(|(item, picked)| match item {
                LanSelectionItemDto::File { .. } => Some(LanFileDto {
                    file_name: picked.file.file_name.clone(),
                    mime_type: picked.file.file_type.clone(),
                    size: picked.file.size,
                }),
                LanSelectionItemDto::Text { .. } => None,
            })
            .collect::<Vec<_>>();
        let preview_png = items
            .iter()
            .zip(picked.iter())
            .find_map(|(item, picked)| match (item, &picked.source) {
                (LanSelectionItemDto::File { .. }, send::SendSource::Path(path))
                    if picked.file.file_type.starts_with("image/") =>
                {
                    util::image_preview_from_path(path)
                }
                _ => None,
            });
        let kind = if text.is_some() && files.is_empty() {
            TransferKind::Text
        } else {
            TransferKind::Files
        };
        let items = items.clone();

        {
            let mut inner = self.inner.lock().unwrap();
            inner.push_transfer(TransferEntry {
                id: transfer_id.clone(),
                direction: "send",
                peer_fingerprint: target.fingerprint.clone(),
                peer_alias: target.alias.clone(),
                status: "active",
                kind,
                label,
                text,
                files,
                items,
                preview_png,
                total_bytes,
                done_bytes: Arc::new(AtomicU64::new(0)),
                error: None,
                created_at_ms: util::now_ms(),
                session_id: None,
                peer_host: Some(target.host.clone()),
                cancel: None,
            });
        }
        emit_state(app, self, &settings);

        let advertised_protocol = self.advertised_protocol();
        send::spawn_send(
            app.clone(),
            self.clone(),
            shared.clone(),
            identity,
            target,
            advertised_protocol,
            transfer_id,
            picked,
        );

        Ok(self.state(&settings))
    }

    // 重新发送一条失败或取消的发送记录：复用记录里保留的原始选择项。
    pub(crate) async fn resend_transfer(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
        transfer_id: &str,
        pin: Option<String>,
    ) -> Result<LanStateDto, AppError> {
        let (fingerprint, items) = {
            let inner = self.inner.lock().unwrap();
            let entry = inner
                .transfers
                .iter()
                .find(|entry| entry.id == transfer_id)
                .ok_or_else(|| AppError::Message("lan_transfer_request_missing".into()))?;
            if entry.direction != "send" || entry.items.is_empty() {
                return Err(AppError::Message(
                    "lan_transfer_resend_unavailable".into(),
                ));
            }
            (entry.peer_fingerprint.clone(), entry.items.clone())
        };

        self.send_items(app, shared, fingerprint, items, pin).await
    }

    pub(crate) fn respond(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
        request_id: &str,
        decision: LanDecision,
    ) -> Result<LanStateDto, AppError> {
        let pending = self.inner.lock().unwrap().pending.remove(request_id);
        let Some(pending) = pending else {
            return Err(AppError::Message("lan_transfer_request_missing".into()));
        };
        let _ = pending.send(decision);

        if matches!(decision, LanDecision::AcceptAndTrust) {
            let settings = shared.settings.lock().unwrap().clone();
            if let Some(entry) = self
                .inner
                .lock()
                .unwrap()
                .incoming
                .clone()
                .filter(|entry| entry.request_id == request_id)
            {
                let mut updated = settings.clone();
                if !updated
                    .lan_trusted_devices
                    .iter()
                    .any(|trusted| trusted.fingerprint == entry.fingerprint)
                {
                    updated.lan_trusted_devices.push(LanTrustedDevice {
                        fingerprint: entry.fingerprint.clone(),
                        alias: entry.alias.clone(),
                        last_seen_at: util::now_ms(),
                    });
                }
                crate::storage::save_settings(&shared.paths, &updated)?;
                *shared.settings.lock().unwrap() = updated;
            }
        }

        let settings = shared.settings.lock().unwrap().clone();
        emit_state(app, self, &settings);
        Ok(self.state(&settings))
    }

    // 取消一次正在进行的传输：发送方向取消请求，接收方向关闭服务端会话。
    pub(crate) async fn cancel_transfer(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
        transfer_id: &str,
    ) -> Result<LanStateDto, AppError> {
        let (is_receive, session_id) = {
            let inner = self.inner.lock().unwrap();
            match inner.transfers.iter().find(|entry| entry.id == transfer_id) {
                Some(entry) => (entry.direction == "receive", entry.session_id.clone()),
                None => (false, None),
            }
        };

        if is_receive {
            if let (Some(session_id), Some(server)) = (session_id, self.server_handle()) {
                let cancelled = server.cancel_v2_session(&session_id).await;
                let mut inner = self.inner.lock().unwrap();
                // 应用主动取消不会触发 SessionEnd，这里自行清掉会话映射。
                inner.sessions.remove(&session_id);
                let still_active = inner
                    .transfers
                    .iter()
                    .any(|entry| entry.id == transfer_id && entry.status == "active");
                if cancelled || still_active {
                    inner.finish_transfer(transfer_id, "cancelled", None);
                }
            }
        } else {
            send::cancel_transfer(self, transfer_id);
        }

        let settings = shared.settings.lock().unwrap().clone();
        emit_state(app, self, &settings);
        Ok(self.state(&settings))
    }

    // 当前运行的 HTTP 服务句柄，用于取消接收会话。
    fn server_handle(&self) -> Option<Arc<ServerHandle>> {
        self.inner
            .lock()
            .unwrap()
            .service
            .as_ref()
            .map(|service| service.server.clone())
    }

    // 监听器或组播永久失败后的自愈：重建整个服务，窗口内重试次数用尽后置为错误状态。
    pub(crate) fn schedule_recovery(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Weak<SharedState>,
        reason: String,
    ) {
        {
            let mut window = self.recovery.lock().unwrap();
            if window.running {
                return;
            }
            let now = util::now_ms();
            if now.saturating_sub(window.started_ms) > RECOVERY_WINDOW_MS {
                window.started_ms = now;
                window.attempts = 0;
            }
            if window.attempts >= MAX_RECOVERY_ATTEMPTS {
                drop(window);
                {
                    let mut inner = self.inner.lock().unwrap();
                    inner.warning = Some("lan_listener_failed".into());
                    inner.error_code = Some("lan_listener_failed".into());
                    inner.error = Some(reason);
                }
                if let Some(shared) = shared.upgrade() {
                    let settings = shared.settings.lock().unwrap().clone();
                    emit_state(app, self, &settings);
                }
                return;
            }
            window.attempts += 1;
            window.running = true;
        }

        let app = app.clone();
        let handle = self.clone();
        let shared = shared.clone();
        tauri::async_runtime::spawn(async move {
            let result = async {
                let shared = shared
                    .upgrade()
                    .ok_or_else(|| anyhow::anyhow!("state unavailable"))?;
                let service = handle.inner.lock().unwrap().service.take();
                if let Some(service) = service {
                    stop_service(service).await;
                }
                {
                    let mut inner = handle.inner.lock().unwrap();
                    // 扫码页模式在服务重建后不再有效，回到普通服务状态。
                    inner.web = WebEntry {
                        mode: "none".into(),
                        url: None,
                        qr_svg: None,
                    };
                    inner.web_paths.clear();
                    inner.sessions.clear();
                    inner.pending.clear();
                    inner.incoming = None;
                    inner.warning = None;
                }
                handle.start(&app, &shared).await.map(|_| ())
            }
            .await;

            if let Err(error) = result {
                let mut inner = handle.inner.lock().unwrap();
                inner.status = "error".into();
                inner.error_code = Some(error_code(&error.to_string()));
                inner.error = Some(error.to_string());
            }
            handle.recovery.lock().unwrap().running = false;
        });
    }

    // 切换浏览器扫码页模式：none / share（分享给手机）/ receive（手机传给我）。
    pub(crate) async fn set_web_mode(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
        mode: &str,
        paths: Vec<String>,
    ) -> Result<LanStateDto, AppError> {
        let settings = shared.settings.lock().unwrap().clone();
        let mode = match mode {
            "share" => "share",
            "receive" => "receive",
            _ => "none",
        };
        let identity = {
            let inner = self.inner.lock().unwrap();
            inner
                .identity
                .clone()
                .ok_or_else(|| AppError::Message("lan_transfer_not_running".into()))?
        };

        let collected = if mode == "share" {
            send::collect_files(&paths)?
        } else {
            Vec::new()
        };

        if let Err(error) = self
            .apply_web_mode(app, shared, &identity, mode, collected)
            .await
        {
            return Err(AppError::from(error));
        }

        emit_state(app, self, &settings);
        Ok(self.state(&settings))
    }

    // 根据消息 ID 取回已接收文件的本地路径。
    pub(crate) fn received_path(&self, id: &str) -> Result<PathBuf, AppError> {
        let inner = self.inner.lock().unwrap();
        inner
            .received
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.path.clone())
            .ok_or_else(|| AppError::Message("lan_transfer_file_not_found".into()))
    }

    // 取回一次图片传输的预览（缩略 PNG）；没有预览时返回 None。
    pub(crate) fn transfer_preview(&self, id: &str) -> Option<Vec<u8>> {
        self.inner
            .lock()
            .unwrap()
            .transfers
            .iter()
            .find(|entry| entry.id == id)
            .and_then(|entry| entry.preview_png.clone())
    }

    pub(crate) fn is_running(&self) -> bool {
        self.inner.lock().unwrap().service.is_some()
    }

    // 本机当前对外广播的协议，与服务端实际监听方式保持一致。
    pub(crate) fn advertised_protocol(&self) -> ProtocolType {
        let mode = self.inner.lock().unwrap().web.mode.clone();
        advertised_device(&mode).0
    }

    // 切换扫码页模式：先切换服务端（协议随模式变化），再重建广播；
    // 任何一步失败都收摊为未运行状态，避免界面显示"运行中"却连不通。
    async fn apply_web_mode(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
        identity: &Arc<LanIdentity>,
        mode: &str,
        files: Vec<send::SendFile>,
    ) -> anyhow::Result<()> {
        let pin = shared.settings.lock().unwrap().lan_transfer_pin.clone();
        if let Err(error) = web_link::restart_server(self, identity, mode, files, pin).await {
            Self::mark_unavailable(self, app, shared, &error.to_string());
            return Err(error);
        }
        let shared_weak = Arc::downgrade(shared);
        if let Err(error) = restart_discovery(app, self, &shared_weak, identity, mode).await {
            Self::mark_unavailable(self, app, shared, &error.to_string());
            return Err(error);
        }
        Ok(())
    }

    // 切换失败后的收摊：停掉可能仍在运行的服务并把原因写入状态。
    fn mark_unavailable(
        this: &Arc<LanTransferHandle>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
        error: &str,
    ) {
        let service = this.inner.lock().unwrap().service.take();
        if let Some(service) = service {
            tauri::async_runtime::spawn(async move {
                stop_service(service).await;
            });
        }
        {
            let mut inner = this.inner.lock().unwrap();
            inner.status = "error".into();
            inner.error_code = Some(error_code(error));
            inner.error = Some(error.to_string());
            inner.web = WebEntry {
                mode: "none".into(),
                url: None,
                qr_svg: None,
            };
            inner.web_paths.clear();
        }
        let settings = shared.settings.lock().unwrap().clone();
        emit_state(app, this, &settings);
    }
}

// 按设置解析本机展示名，为空时回退到主机名。
fn resolve_alias(settings: &AppSettings) -> String {
    settings
        .lan_device_alias
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(default_alias)
}

// 把内部设备记录转换成前端 DTO，online 由调用方按当前发现列表判定。
fn device_dto(device: &DeviceEntry, settings: &AppSettings, online: bool) -> LanDeviceDto {
    LanDeviceDto {
        fingerprint: device.fingerprint.clone(),
        alias: device.alias.clone(),
        device_model: device.device_model.clone(),
        device_type: device.device_type.clone(),
        host: device.host.clone(),
        port: device.port,
        protocol: device.protocol.as_str().to_string(),
        trusted: settings
            .lan_trusted_devices
            .iter()
            .any(|entry| entry.fingerprint == device.fingerprint),
        online,
        last_seen_ms: device.last_seen_ms,
    }
}

fn default_alias() -> String {
    alias_candidates()
        .into_iter()
        .find_map(|value| normalize_hostname(&value))
        .unwrap_or_else(|| "Power Paste".to_string())
}

// 收集默认名候选：Windows 的 COMPUTERNAME、部分 shell 的 HOSTNAME，最后补上系统主机名。
fn alias_candidates() -> Vec<String> {
    let mut candidates = Vec::new();
    for key in ["COMPUTERNAME", "HOSTNAME"] {
        if let Ok(value) = std::env::var(key) {
            candidates.push(value);
        }
    }
    if let Some(value) = system_hostname() {
        candidates.push(value);
    }
    candidates
}

// 系统主机名：macOS 的 GUI 进程不继承 shell 环境变量，必须向系统查询；Linux 读内核主机名文件。
fn system_hostname() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_foundation::NSProcessInfo;

        Some(NSProcessInfo::processInfo().hostName().to_string())
    }

    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/sys/kernel/hostname").ok()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

// 规范化主机名：去掉首尾空白与 `.local` 后缀，过滤空值与 localhost，避免设备名退化成无效值。
fn normalize_hostname(value: &str) -> Option<String> {
    let trimmed = strip_local_suffix(value.trim()).trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("localhost") {
        return None;
    }
    Some(trimmed.to_string())
}

// 忽略大小写地去掉 `.local` 后缀；按原字符串长度截断，保证索引不会落在字符边界之外。
fn strip_local_suffix(value: &str) -> &str {
    match value.to_ascii_lowercase().strip_suffix(".local") {
        Some(prefix) => &value[..prefix.len()],
        None => value,
    }
}

// 解析本次发送使用的 PIN：优先使用用户刚输入的，其次复用本设备已成功验证过的。
fn resolve_pin(inner: &Inner, fingerprint: &str, pin: Option<String>) -> Option<String> {
    pin.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| inner.device_pins.get(fingerprint).cloned())
}

// 把底层错误归为稳定错误码，供前端本地化展示；未识别时回退到通用码。
pub(crate) fn error_code(error: &str) -> String {
    let lower = error.to_lowercase();
    if lower.contains("address already in use")
        || lower.contains("os error 10048")
        || lower.contains("os error 98")
        || lower.contains("only one usage of each socket address")
    {
        "lan_port_in_use".into()
    } else if lower.contains("lan_transfer_not_running") {
        "lan_transfer_not_running".into()
    } else if lower.contains("no network interface available for multicast") {
        "lan_multicast_unavailable".into()
    } else {
        "lan_service_failed".into()
    }
}

// 启动服务所需的后台任务：HTTP 服务、组播发现与事件转发。
async fn build_service(
    app: &AppHandle,
    handle: &Arc<LanTransferHandle>,
    shared: &Arc<SharedState>,
    identity: Arc<LanIdentity>,
) -> anyhow::Result<Service> {
    let (server_tx, server_rx) = mpsc::channel::<ServerEventV2>(16);
    let (web_tx, web_rx) = mpsc::channel::<WebSendEvent>(16);
    let (server_stop_tx, server_stop_rx) = oneshot::channel::<()>();
    let pin = shared.settings.lock().unwrap().lan_transfer_pin.clone();

    let server = start_with_port(
        identity.port,
        Some(identity.tls_config()),
        identity.client_info(),
        None,
        Some(ServerConfigV2 {
            pin,
            verify_checksums: true,
            event_tx: server_tx.clone(),
        }),
        Some(WebConfig {
            send: None,
            upload: false,
            i18n: Default::default(),
            pages: Default::default(),
        }),
        server_stop_rx,
    )
    .await?;

    let (discovery, discovery_stop_tx) =
        start_discovery(app, handle, &Arc::downgrade(shared), &identity, "none").await?;
    receive::spawn_server_events(
        app.clone(),
        handle.clone(),
        Arc::downgrade(shared),
        server_rx,
    );
    web_link::spawn_web_events(app.clone(), handle.clone(), Arc::downgrade(shared), web_rx);

    // 启动后立即广播一次，让网内设备马上发现本机。
    let announce = discovery.clone();
    tokio::spawn(async move {
        announce.announce().await;
    });

    Ok(Service {
        server: Arc::new(server),
        server_stop: Some(server_stop_tx),
        discovery,
        discovery_stop: Some(discovery_stop_tx),
        server_tx,
        web_tx,
    })
}

// 启动一次组播发现并把事件接入应用状态；返回句柄与停止信号发送端。
async fn start_discovery(
    app: &AppHandle,
    handle: &Arc<LanTransferHandle>,
    shared: &Weak<SharedState>,
    identity: &Arc<LanIdentity>,
    mode: &str,
) -> anyhow::Result<(Arc<DiscoveryHandle>, oneshot::Sender<()>)> {
    let (protocol, download) = advertised_device(mode);
    let (discovery_tx, discovery_rx) = mpsc::channel::<DiscoveryEvent>(16);
    let (discovery_stop_tx, discovery_stop_rx) = oneshot::channel::<()>();
    let discovery = Arc::new(
        localsend::discovery::start(
            DiscoveryConfig {
                group: DEFAULT_MULTICAST_GROUP,
                group_v6: Some(DEFAULT_MULTICAST_GROUP_V6),
                port: DEFAULT_PORT,
                interface_filter: InterfaceFilter::default(),
                device: identity.multicast_device(protocol, download),
                identity: DeviceIdentity {
                    cert_pem: identity.cert_pem.clone(),
                    private_key_pem: identity.key_pem.clone(),
                },
                timeout: client_timeout_for(ClientPurpose::Discovery)
                    .unwrap_or(DEFAULT_DISCOVERY_TIMEOUT),
                event_tx: Some(discovery_tx),
            },
            discovery_stop_rx,
        )
        .await,
    );

    spawn_discovery_events(
        app.clone(),
        handle.clone(),
        shared.clone(),
        discovery.clone(),
        discovery_rx,
    );
    Ok((discovery, discovery_stop_tx))
}

// 按扫码页模式重建组播广播：协议与下载能力必须切换到服务端实际状态，
// 否则对端会按错误协议连接（例如页面打开时按 https 连接明文 HTTP 服务）。
async fn restart_discovery(
    app: &AppHandle,
    handle: &Arc<LanTransferHandle>,
    shared: &Weak<SharedState>,
    identity: &Arc<LanIdentity>,
    mode: &str,
) -> anyhow::Result<()> {
    let (old_discovery, old_stop) = {
        let mut inner = handle.inner.lock().unwrap();
        let service = inner
            .service
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("lan_transfer_not_running"))?;
        (service.discovery.clone(), service.discovery_stop.take())
    };

    if let Some(stop) = old_stop {
        let _ = stop.send(());
    }
    let _ = tokio::time::timeout(Duration::from_secs(3), old_discovery.wait_stopped()).await;

    let (discovery, stop_tx) = start_discovery(app, handle, shared, identity, mode).await?;
    let mut stop_tx = Some(stop_tx);
    let stored = {
        let mut inner = handle.inner.lock().unwrap();
        match inner.service.as_mut() {
            Some(service) => {
                service.discovery = discovery.clone();
                service.discovery_stop = stop_tx.take();
                true
            }
            None => false,
        }
    };
    if !stored {
        if let Some(stop) = stop_tx.take() {
            let _ = stop.send(());
        }
        anyhow::bail!("lan_transfer_not_running");
    }

    // 立即广播一次，让网内设备按新的协议与能力更新本机信息。
    tokio::spawn(async move {
        discovery.announce().await;
    });
    Ok(())
}

async fn stop_service(mut service: Service) {
    if let Some(stop) = service.server_stop.take() {
        let _ = stop.send(());
    }
    if let Some(stop) = service.discovery_stop.take() {
        let _ = stop.send(());
    }
    let _ = tokio::time::timeout(Duration::from_secs(3), service.server.wait_stopped()).await;
    let _ = tokio::time::timeout(Duration::from_secs(3), service.discovery.wait_stopped()).await;
}

// 组播/HTTP 探测确认的设备都会进入列表。
fn spawn_discovery_events(
    app: AppHandle,
    handle: Arc<LanTransferHandle>,
    shared: Weak<SharedState>,
    discovery: Arc<DiscoveryHandle>,
    mut rx: mpsc::Receiver<DiscoveryEvent>,
) {
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                DiscoveryEvent::Discovered { device } | DiscoveryEvent::Updated { device } => {
                    let Some(channel) = device.channel.http() else {
                        continue;
                    };
                    handle.upsert_device(
                        device.fingerprint.clone(),
                        device.alias.clone(),
                        device.device_model.clone(),
                        device.device_type.as_ref().map(device_type_label),
                        channel.host.clone(),
                        channel.port,
                        channel.protocol,
                    );
                    let _ = &discovery;
                    emit_state_with(&app, &handle, &shared);
                }
                DiscoveryEvent::MulticastFailed => {
                    handle.inner.lock().unwrap().warning =
                        Some("lan_multicast_unavailable".to_string());
                    emit_state_with(&app, &handle, &shared);
                    // 组播套接字已失效：重建服务，否则本机再也听不到/发不出广播。
                    handle.schedule_recovery(
                        &app,
                        &shared,
                        "lan_multicast_unavailable".to_string(),
                    );
                }
            }
        }
    });
}

// 供接收/发送链路回填设备列表（对端主动注册或发送时确认）。
pub(crate) fn confirm_device(
    handle: &Arc<LanTransferHandle>,
    info: &localsend::http::dto_v2::RegisterDtoV2,
    host: String,
    fingerprint: String,
) {
    handle.upsert_device(
        fingerprint,
        info.alias.clone(),
        info.device_model.clone(),
        info.device_type.as_ref().map(device_type_label),
        host,
        info.port,
        info.protocol,
    );
}

pub(crate) fn device_type_label(value: &localsend::model::discovery::DeviceType) -> String {
    match value {
        localsend::model::discovery::DeviceType::Mobile => "mobile",
        localsend::model::discovery::DeviceType::Desktop => "desktop",
        localsend::model::discovery::DeviceType::Web => "web",
        localsend::model::discovery::DeviceType::Headless => "headless",
        localsend::model::discovery::DeviceType::Server => "server",
    }
    .to_string()
}

// 用当前设置推送完整状态。
pub(crate) fn emit_state_with(
    app: &AppHandle,
    handle: &Arc<LanTransferHandle>,
    shared: &Weak<SharedState>,
) {
    let Some(shared) = shared.upgrade() else {
        return;
    };
    let settings = shared.settings.lock().unwrap().clone();
    emit_state(app, handle, &settings);
}

pub(crate) fn emit_state(app: &AppHandle, handle: &Arc<LanTransferHandle>, settings: &AppSettings) {
    let dto = handle.state(settings);
    let _ = app.emit(LAN_STATE_EVENT, dto);
}

// 判断某指纹是否已被信任。
pub(crate) fn is_trusted(settings: &AppSettings, fingerprint: &str) -> bool {
    settings
        .lan_trusted_devices
        .iter()
        .any(|entry| entry.fingerprint == fingerprint)
}

// 记住用户最近一次主动选择的扫描网段，供下次刷新时高亮。
fn remember_scan_subnet(shared: &Arc<SharedState>, cidr: &str) -> Result<(), AppError> {
    let mut settings = shared.settings.lock().unwrap().clone();
    if settings.lan_scan_last_subnet.as_deref() == Some(cidr) {
        return Ok(());
    }

    settings.lan_scan_last_subnet = Some(cidr.to_string());
    crate::storage::save_settings(&shared.paths, &settings)?;
    *shared.settings.lock().unwrap() = settings;
    Ok(())
}

// 设置变更后同步局域网服务：开关变化时启停，设备名或 PIN 变化时重启以更新广播与校验。
pub(crate) fn apply_settings_change(
    app: AppHandle,
    shared: Arc<SharedState>,
    enabled: bool,
    alias_changed: bool,
    pin_changed: bool,
) {
    let handle = shared.lan_transfer.clone();
    // 设置更新走的是同步 tauri command，必须用 tauri 的异步运行时派发任务，
    // 否则在没有 tokio 上下文的线程上调用 tokio::spawn 会直接 panic。
    tauri::async_runtime::spawn(async move {
        let running = handle.is_running();
        if !enabled {
            if running {
                let _ = handle.stop(&app, &shared).await;
            }
            return;
        }
        if running {
            if alias_changed || pin_changed {
                let _ = handle.stop(&app, &shared).await;
                let _ = handle.start(&app, &shared).await;
            }
            return;
        }
        let _ = handle.start(&app, &shared).await;
    });
}

#[cfg(test)]
mod tests {
    use super::{
        advertised_device, client_timeout_for, normalize_hostname, ClientPurpose, DeviceEntry,
        LanSelectionItemDto, LanTransferHandle, TransferEntry, TransferKind,
    };
    use crate::models::AppSettings;
    use localsend::model::discovery::ProtocolType;
    use std::sync::{atomic::AtomicU64, Arc};

    // 造一条测试用设备记录。
    fn device_entry(fingerprint: &str) -> DeviceEntry {
        DeviceEntry {
            fingerprint: fingerprint.to_string(),
            alias: "Peer".into(),
            device_model: Some("Model".into()),
            device_type: Some("desktop".into()),
            host: "192.168.1.20".into(),
            port: 53317,
            protocol: ProtocolType::Https,
            last_seen_ms: 1,
        }
    }

    // 造一条测试用传输记录。
    fn transfer_entry(kind: TransferKind, direction: &'static str) -> TransferEntry {
        TransferEntry {
            id: "transfer-1".into(),
            direction,
            peer_fingerprint: "peer-1".into(),
            peer_alias: "Peer".into(),
            status: "active",
            kind,
            label: "text".into(),
            text: (kind == TransferKind::Text).then(|| "hello".to_string()),
            files: Vec::new(),
            items: Vec::new(),
            preview_png: None,
            total_bytes: 5,
            done_bytes: Arc::new(AtomicU64::new(0)),
            error: None,
            created_at_ms: 1_700_000_000_000,
            session_id: None,
            peer_host: None,
            cancel: None,
        }
    }

    #[test]
    fn advertises_the_protocol_the_server_actually_serves() {
        // 扫码页打开时服务端只提供明文 HTTP，广播必须同步切换，否则对端按 https 连接。
        assert_eq!(advertised_device("none"), (ProtocolType::Https, false));
        assert_eq!(advertised_device("share"), (ProtocolType::Http, true));
        assert_eq!(advertised_device("receive"), (ProtocolType::Http, false));
        assert_eq!(advertised_device("unknown"), (ProtocolType::Https, false));
    }

    #[test]
    fn never_caps_transfer_requests_with_a_total_timeout() {
        // 对端人工确认与大体量上传都可能远超 30 秒，传输链路不能设置总超时。
        assert_eq!(client_timeout_for(ClientPurpose::Transfer), None);
        assert!(client_timeout_for(ClientPurpose::Discovery).is_some());
    }

    #[test]
    fn normalizes_hostnames_for_the_default_device_name() {
        // macOS 的主机名带 `.local`，Windows 的名称可能带空白，都要归一成可直接展示的名字。
        assert_eq!(
            normalize_hostname("MacBook-Pro.local"),
            Some("MacBook-Pro".to_string())
        );
        assert_eq!(
            normalize_hostname("  DESKTOP-ABC  "),
            Some("DESKTOP-ABC".to_string())
        );
        assert_eq!(normalize_hostname("host.LOCAL"), Some("host".to_string()));
        assert_eq!(normalize_hostname(""), None);
        assert_eq!(normalize_hostname("   "), None);
        assert_eq!(normalize_hostname("localhost"), None);
    }

    #[test]
    fn always_resolves_a_default_device_name() {
        // 任何平台上都不能出现空设备名，否则对端列表里会出现无名设备。
        assert!(!super::default_alias().is_empty());
    }

    #[test]
    fn keeps_known_peers_after_the_service_stops() {
        // 会话列表需要保留离线对端：服务停止清空在线设备后，peers 仍应可用且标记为离线。
        let handle = LanTransferHandle::new();
        handle.upsert_device(
            "peer-1".into(),
            "Peer".into(),
            None,
            None,
            "192.168.1.20".into(),
            53317,
            ProtocolType::Https,
        );
        handle.inner.lock().unwrap().devices.clear();

        let state = handle.state(&AppSettings::default());
        assert!(state.devices.is_empty());
        assert_eq!(state.peers.len(), 1);
        assert!(!state.peers[0].online);

        handle
            .inner
            .lock()
            .unwrap()
            .devices
            .push(device_entry("peer-1"));
        let state = handle.state(&AppSettings::default());
        assert!(state.peers[0].online);
    }

    #[test]
    fn exposes_chat_metadata_on_transfer_records() {
        // 会话气泡依赖时间戳、对端指纹与文本正文，缺失会让历史消息退化成一行状态。
        let handle = LanTransferHandle::new();
        handle
            .inner
            .lock()
            .unwrap()
            .transfers
            .push(transfer_entry(TransferKind::Text, "receive"));

        let state = handle.state(&AppSettings::default());
        let transfer = &state.transfers[0];
        assert_eq!(transfer.kind, "text");
        assert_eq!(transfer.peer_fingerprint, "peer-1");
        assert_eq!(transfer.created_at_ms, 1_700_000_000_000);
        assert_eq!(transfer.text.as_deref(), Some("hello"));
        assert!(!transfer.resendable);
    }

    #[test]
    fn only_send_records_with_stored_items_can_be_resent() {
        // 接收方向没有本地内容，或发送时没有保留选择项，都不允许重发。
        let handle = LanTransferHandle::new();
        {
            let mut inner = handle.inner.lock().unwrap();
            let mut without_items = transfer_entry(TransferKind::Text, "send");
            without_items.id = "transfer-without-items".into();
            inner.transfers.push(without_items);

            let mut resentable = transfer_entry(TransferKind::Text, "send");
            resentable.id = "transfer-resentable".into();
            resentable.items = vec![LanSelectionItemDto::text("hello".into())];
            inner.transfers.push(resentable);
        }

        let state = handle.state(&AppSettings::default());
        let resentable = state
            .transfers
            .iter()
            .find(|entry| entry.id == "transfer-resentable")
            .expect("resentable transfer missing");
        let plain = state
            .transfers
            .iter()
            .find(|entry| entry.id == "transfer-without-items")
            .expect("plain transfer missing");
        assert!(resentable.resendable);
        assert!(!plain.resendable);
    }

    #[test]
    fn folds_single_text_transfers_into_messages() {
        // 单文件文本传输折叠成消息气泡：显示正文而不是 txt 文件名。
        let handle = LanTransferHandle::new();
        {
            let mut inner = handle.inner.lock().unwrap();
            let mut text_package = transfer_entry(TransferKind::Files, "receive");
            text_package.files = vec![super::LanFileDto {
                file_name: "PowerPaste-Text-1.txt".into(),
                mime_type: "text/plain".into(),
                size: 5,
            }];
            text_package.text = None;
            inner.transfers.push(text_package);
            inner.mark_transfer_as_text("transfer-1", "hello");
        }

        let state = handle.state(&AppSettings::default());
        let transfer = &state.transfers[0];
        assert_eq!(transfer.kind, "text");
        assert_eq!(transfer.text.as_deref(), Some("hello"));
        assert!(transfer.files.is_empty());
    }

    #[test]
    fn keeps_mixed_transfers_as_file_lists() {
        // 一条传输里还有其它文件时不能折叠，否则用户会看不到文件清单。
        let handle = LanTransferHandle::new();
        {
            let mut inner = handle.inner.lock().unwrap();
            let mut mixed = transfer_entry(TransferKind::Files, "receive");
            mixed.files = vec![
                super::LanFileDto {
                    file_name: "message.txt".into(),
                    mime_type: "text/plain".into(),
                    size: 5,
                },
                super::LanFileDto {
                    file_name: "report.pdf".into(),
                    mime_type: "application/pdf".into(),
                    size: 10,
                },
            ];
            inner.transfers.push(mixed);
            inner.mark_transfer_as_text("transfer-1", "hello");
        }

        let state = handle.state(&AppSettings::default());
        assert_eq!(state.transfers[0].kind, "files");
        assert_eq!(state.transfers[0].files.len(), 2);
        assert!(state.transfers[0].text.is_none());
    }

    #[test]
    fn exposes_image_previews_on_demand() {
        // 预览不进状态快照，只通过 transfer_preview 按需取回。
        let handle = LanTransferHandle::new();
        {
            let mut inner = handle.inner.lock().unwrap();
            inner
                .transfers
                .push(transfer_entry(TransferKind::Files, "send"));
            inner.set_transfer_preview("transfer-1", Some(vec![1, 2, 3]));
        }

        let state = handle.state(&AppSettings::default());
        assert!(state.transfers[0].has_preview);
        assert_eq!(handle.transfer_preview("transfer-1"), Some(vec![1, 2, 3]));
        assert_eq!(handle.transfer_preview("missing"), None);
    }
}
