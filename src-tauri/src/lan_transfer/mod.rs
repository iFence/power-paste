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
        DiscoveryConfig, DiscoveryEvent, DiscoveryHandle, DeviceIdentity, HttpChannel,
        DEFAULT_DISCOVERY_TIMEOUT,
    },
    http::server::{
        start_with_port,
        v2::ServerEventV2,
        web::{WebConfig, WebSendEvent},
        ServerConfigV2, ServerHandle,
    },
    model::discovery::ProtocolType,
    multicast::{DEFAULT_MULTICAST_GROUP, DEFAULT_MULTICAST_GROUP_V6, DEFAULT_PORT},
    util::interface::{local_interface_addresses, InterfaceFilter},
};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::models::{AppError, AppSettings, LanTrustedDevice, SharedState};

mod identity;
mod receive;
mod send;
mod text_package;
mod util;
mod web_link;

use identity::LanIdentity;
pub(crate) use text_package::{is_text_package, TEXT_RESTORE_MAX_BYTES};
pub(crate) use util::validate_download_dir;

// 前端状态事件：任意变化都会推送完整状态，前端直接替换即可。
pub(crate) const LAN_STATE_EVENT: &str = "lan-transfer-state";

// 等待用户确认接收的超时时间，超时按拒绝处理，避免对端一直挂起。
pub(crate) const INCOMING_TIMEOUT: Duration = Duration::from_secs(45);

// 传输记录与已接收文件在内存中的上限，避免长时间运行后无限增长（磁盘文件不删除）。
const MAX_TRANSFER_ENTRIES: usize = 50;
const MAX_RECEIVED_ENTRIES: usize = 200;

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
    pub(crate) transfers: Vec<LanTransferDto>,
    pub(crate) incoming: Option<LanIncomingRequestDto>,
    pub(crate) received_files: Vec<LanReceivedFileDto>,
    pub(crate) web_mode: String,
    pub(crate) web_url: Option<String>,
    pub(crate) web_qr_svg: Option<String>,
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
    pub(crate) last_seen_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanTransferDto {
    pub(crate) id: String,
    pub(crate) direction: String,
    pub(crate) peer_alias: String,
    pub(crate) status: String,
    pub(crate) label: String,
    pub(crate) total_bytes: u64,
    pub(crate) done_bytes: u64,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanIncomingFileDto {
    pub(crate) file_name: String,
    pub(crate) size: u64,
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
    pub(crate) files: Vec<LanIncomingFileDto>,
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

struct TransferEntry {
    id: String,
    direction: &'static str,
    peer_alias: String,
    status: &'static str,
    label: String,
    total_bytes: u64,
    done_bytes: Arc<AtomicU64>,
    error: Option<String>,
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
    files: Vec<(String, u64)>,
    total_bytes: u64,
    text_message: Option<String>,
}

struct ReceivedEntry {
    id: String,
    file_name: String,
    path: PathBuf,
    size: u64,
    from_alias: String,
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
}

pub(crate) struct LanTransferHandle {
    inner: Mutex<Inner>,
    recovery: Mutex<RecoveryWindow>,
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
        }
    }

    // 组装前端状态快照。
    pub(crate) fn state(&self, settings: &AppSettings) -> LanStateDto {
        let inner = self.inner.lock().unwrap();
        let identity = inner.identity.as_ref();
        let alias = identity
            .map(|value| value.alias.clone())
            .unwrap_or_else(|| resolve_alias(settings));

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
                .map(|device| LanDeviceDto {
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
                    last_seen_ms: device.last_seen_ms,
                })
                .collect(),
            transfers: inner
                .transfers
                .iter()
                .map(|entry| LanTransferDto {
                    id: entry.id.clone(),
                    direction: entry.direction.to_string(),
                    peer_alias: entry.peer_alias.clone(),
                    status: entry.status.to_string(),
                    label: entry.label.clone(),
                    total_bytes: entry.total_bytes,
                    done_bytes: entry.done_bytes.load(Ordering::Relaxed),
                    error: entry.error.clone(),
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
                    .map(|(file_name, size)| LanIncomingFileDto {
                        file_name: file_name.clone(),
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
                    received_at_ms: entry.received_at_ms,
                })
                .collect(),
            web_mode: inner.web.mode.clone(),
            web_url: inner.web.url.clone(),
            web_qr_svg: inner.web.qr_svg.clone(),
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

        let settings = shared.settings.lock().unwrap().clone();
        emit_state(app, self, &settings);
        Ok(self.state(&settings))
    }

    // 主动刷新一次设备发现：先广播，再探测已知地址，必要时扫描网段。
    pub(crate) async fn refresh(
        self: &Arc<Self>,
        app: &AppHandle,
        shared: &Arc<SharedState>,
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

        let interface_ips = local_interface_addresses(&InterfaceFilter::default()).unwrap_or_default();
        let known_channels: Vec<HttpChannel> = {
            let inner = self.inner.lock().unwrap();
            inner
                .devices
                .iter()
                .map(|device| HttpChannel {
                    host: device.host.clone(),
                    port: device.port,
                    protocol: device.protocol,
                })
                .collect()
        };

        let _ = discovery
            .discover_staged(
                known_channels,
                interface_ips,
                DEFAULT_PORT,
                ProtocolType::Https,
                Duration::from_secs(1),
            )
            .await;

        emit_state(app, self, &settings);
        Ok(self.state(&settings))
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
                    let mut inner = self.inner.lock().unwrap();
                    inner.upsert_device(
                        device.device.fingerprint.clone(),
                        device.device.alias.clone(),
                        device.device.device_model.clone(),
                        device
                            .device
                            .device_type
                            .as_ref()
                            .map(device_type_label),
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

        {
            let mut inner = self.inner.lock().unwrap();
            inner.push_transfer(TransferEntry {
                id: transfer_id.clone(),
                direction: "send",
                peer_alias: target.alias.clone(),
                status: "active",
                label,
                total_bytes,
                done_bytes: Arc::new(AtomicU64::new(0)),
                error: None,
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
                peer_alias: target.alias.clone(),
                status: "active",
                label: "text".into(),
                total_bytes: message.file.size,
                done_bytes: Arc::new(AtomicU64::new(text.len() as u64)),
                error: None,
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

        if let Err(error) = self.apply_web_mode(app, shared, &identity, mode, collected).await {
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

fn default_alias() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .map(|value| value.trim_end_matches(".local").to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Power Paste".to_string())
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
                    {
                        let mut inner = handle.inner.lock().unwrap();
                        inner.upsert_device(
                            device.fingerprint.clone(),
                            device.alias.clone(),
                            device.device_model.clone(),
                            device.device_type.as_ref().map(device_type_label),
                            channel.host.clone(),
                            channel.port,
                            channel.protocol,
                        );
                    }
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
    let mut inner = handle.inner.lock().unwrap();
    inner.upsert_device(
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
    use super::{advertised_device, client_timeout_for, ClientPurpose};
    use localsend::model::discovery::ProtocolType;

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
}
