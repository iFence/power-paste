//! 网段扫描：枚举本机网卡所在 /24 网段，并按网段主动探测 LocalSend 设备。
//!
//! 与官方 LocalSend 的“刷新查找设备”保持一致：先做廉价的组播广播与已知地址
//! 探测，只有在局域网里得不到任何确认时才回退扫网段；用户也可以直接指定本机
//! 某个网段单独扫描。相比官方实现额外提供逐主机进度与即时取消。

use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr},
    sync::{atomic::Ordering, Arc, Weak},
    time::{Duration, Instant},
};

use futures_util::{
    pin_mut,
    stream::{self, StreamExt},
};
use localsend::{
    discovery::{DiscoveryHandle, HttpChannel, StatefulDevice, DEFAULT_DISCOVERY_TIMEOUT},
    http::{
        client::v2::LsHttpClientV2,
        dto_v2::{RegisterDtoV2, RegisterResponseDtoV2},
    },
    model::discovery::ProtocolType,
};
use serde::Serialize;
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use super::{device_type_label, emit_state_with, identity::LanIdentity, LanTransferHandle};
use crate::models::{AppError, SharedState};

// 每个网段并发探测的主机数，与上游 LocalSend 的传统 HTTP 发现保持一致。
const SCAN_CONCURRENCY: usize = 50;

// 扫描进度推送节流：与传输进度一致，最多每 200 毫秒推一次完整状态。
const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(200);

// 智能刷新在已知地址探测完成后的等待窗口：窗口内出现任何确认就不再回退扫网段。
const SMART_SCAN_GRACE: Duration = Duration::from_secs(1);

// /24 网段里的可用主机地址固定为 1..=254，排除网络地址与广播地址。
const SUBNET_FIRST_HOST: u8 = 1;
const SUBNET_LAST_HOST: u8 = 254;

// 前端可见的本机网段候选。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanSubnetDto {
    pub(crate) cidr: String,
    pub(crate) interface: String,
    pub(crate) address: String,
    // 虚拟网卡（容器/虚拟机/隧道）：仅用于前端判断“是否只有一个真实网段”。
    pub(crate) virtual_interface: bool,
}

// 网段候选列表与上次选择，供刷新菜单渲染。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanSubnetsDto {
    pub(crate) last_subnet: Option<String>,
    pub(crate) subnets: Vec<LanSubnetDto>,
}

// 扫描进度快照：结束后保留最后一次结果，直到下一次扫描或服务停止。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanScanDto {
    pub(crate) running: bool,
    pub(crate) subnets: Vec<LanScanSubnetDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanScanSubnetDto {
    pub(crate) cidr: String,
    pub(crate) interface: String,
    pub(crate) scanned: u32,
    pub(crate) total: u32,
    pub(crate) found: u32,
    pub(crate) done: bool,
}

// 本次扫描的范围：智能刷新按需回退，或用户显式选定的网段。
pub(crate) enum ScanScope {
    Auto,
    Subnets(Vec<ScanSubnet>),
}

// 一个已校验的 /24 扫描目标。
pub(crate) struct ScanSubnet {
    pub(crate) cidr: String,
    pub(crate) base: Ipv4Addr,
}

// 当前（或最近一次）扫描的运行时状态，随完整状态一起推给前端。
pub(crate) struct ScanState {
    pub(crate) generation: u64,
    pub(crate) running: bool,
    pub(crate) cancel: CancellationToken,
    pub(crate) subnets: Vec<ScanSubnetState>,
}

pub(crate) struct ScanSubnetState {
    pub(crate) cidr: String,
    pub(crate) interface: String,
    pub(crate) scanned: u32,
    pub(crate) total: u32,
    pub(crate) found: u32,
    pub(crate) done: bool,
}

impl ScanState {
    pub(crate) fn to_dto(&self) -> LanScanDto {
        LanScanDto {
            running: self.running,
            subnets: self
                .subnets
                .iter()
                .map(|subnet| LanScanSubnetDto {
                    cidr: subnet.cidr.clone(),
                    interface: subnet.interface.clone(),
                    scanned: subnet.scanned,
                    total: subnet.total,
                    found: subnet.found,
                    done: subnet.done,
                })
                .collect(),
        }
    }
}

// 本机某张网卡所在的 /24 网段。
struct LocalSubnet {
    cidr: String,
    base: Ipv4Addr,
    address: Ipv4Addr,
    interface: String,
    virtual_interface: bool,
}

// 组装扫描期间需要随任务一起携带的本机网络信息，避免任务里再做可能失败的枚举。
struct LocalNetwork {
    subnets: Vec<ScanSubnet>,
    interface_names: HashMap<String, String>,
    own_addresses: HashSet<Ipv4Addr>,
}

// 列出本机网段候选与上次选择，供刷新菜单渲染。
pub(crate) fn list_subnets(last: Option<&str>) -> Result<LanSubnetsDto, AppError> {
    let subnets = local_subnets()?;
    Ok(LanSubnetsDto {
        last_subnet: last.map(ToString::to_string),
        subnets: subnets
            .into_iter()
            .map(|entry| LanSubnetDto {
                cidr: entry.cidr,
                interface: entry.interface,
                address: entry.address.to_string(),
                virtual_interface: entry.virtual_interface,
            })
            .collect(),
    })
}

// 校验前端选定的网段：必须是 /24，返回去重后的扫描目标。
pub(crate) fn normalize_subnets(values: &[String]) -> Result<Vec<ScanSubnet>, AppError> {
    let mut seen = HashSet::new();
    let mut subnets = Vec::new();
    for value in values {
        let base = parse_subnet(value)
            .ok_or_else(|| AppError::Message("lan_scan_invalid_subnet".into()))?;
        let cidr = cidr_of(base);
        if seen.insert(cidr.clone()) {
            subnets.push(ScanSubnet { cidr, base });
        }
    }

    if subnets.is_empty() {
        return Err(AppError::Message("lan_scan_no_subnet".into()));
    }
    Ok(subnets)
}

// 启动一次扫描；同一时刻只保留最新一次扫描，旧的会被取消。
// 返回本次实际扫描的网段列表，供调用方记录最近选择。
pub(crate) fn start(
    handle: &Arc<LanTransferHandle>,
    app: &AppHandle,
    shared: &Arc<SharedState>,
    scope: ScanScope,
    port: u16,
) -> Result<Vec<String>, AppError> {
    // 服务未运行时不扫描：广播、已知地址探测与探测端口都依赖运行中的服务。
    let identity = {
        let inner = handle.inner.lock().unwrap();
        if inner.service.is_none() {
            return Err(AppError::Message("lan_transfer_not_running".into()));
        }
        inner
            .identity
            .clone()
            .ok_or_else(|| AppError::Message("lan_transfer_not_running".into()))?
    };

    // 探测用 HTTPS 读取对端证书指纹；注册请求里声明本机实际对外服务的协议。
    let client = LsHttpClientV2::try_new(
        &identity.key_pem,
        &identity.cert_pem,
        None,
        Some(DEFAULT_DISCOVERY_TIMEOUT),
    )
    .map_err(|error| AppError::Message(error.to_string()))?;
    let dto = identity.register_dto(handle.advertised_protocol());

    // 所有可能失败的枚举都在命令返回前完成，扫描任务本身不再有失败分支。
    let network = local_network()?;
    let LocalNetwork {
        subnets,
        interface_names,
        own_addresses,
    } = network;

    let (scope, rows, resolved) = match scope {
        ScanScope::Auto => (ScanScope::Auto, Vec::new(), Vec::new()),
        ScanScope::Subnets(requested) => {
            let rows = build_rows(&requested, &interface_names, &own_addresses);
            let resolved = requested.iter().map(|subnet| subnet.cidr.clone()).collect();
            (ScanScope::Subnets(requested), rows, resolved)
        }
    };

    let cancel = CancellationToken::new();
    let (generation, previous) = {
        let mut inner = handle.inner.lock().unwrap();
        let generation = inner
            .scan
            .as_ref()
            .map(|scan| scan.generation + 1)
            .unwrap_or(1);
        let previous = inner.scan.replace(ScanState {
            generation,
            running: true,
            cancel: cancel.clone(),
            subnets: rows,
        });
        (generation, previous)
    };
    if let Some(previous) = previous {
        previous.cancel.cancel();
    }

    let task_handle = handle.clone();
    let task_app = app.clone();
    let task_shared = Arc::downgrade(shared);
    tauri::async_runtime::spawn(async move {
        run_scan(
            task_handle,
            task_app,
            task_shared,
            generation,
            identity,
            dto,
            client,
            scope,
            LocalNetwork {
                subnets,
                interface_names,
                own_addresses,
            },
            port,
            cancel,
        )
        .await;
    });

    Ok(resolved)
}

// 取消正在进行的扫描；返回是否真的取消了。
pub(crate) fn cancel(handle: &LanTransferHandle) -> bool {
    let mut inner = handle.inner.lock().unwrap();
    match inner.scan.as_mut() {
        Some(scan) if scan.running => {
            scan.running = false;
            scan.cancel.cancel();
            true
        }
        _ => false,
    }
}

// 停服务时一并取消扫描并清空进度快照，避免下次启动显示陈旧的网段。
pub(crate) fn clear(handle: &LanTransferHandle) {
    if let Some(scan) = handle.inner.lock().unwrap().scan.take() {
        scan.cancel.cancel();
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_scan(
    handle: Arc<LanTransferHandle>,
    app: AppHandle,
    shared: Weak<SharedState>,
    generation: u64,
    identity: Arc<LanIdentity>,
    dto: RegisterDtoV2,
    client: LsHttpClientV2,
    scope: ScanScope,
    network: LocalNetwork,
    port: u16,
    cancel: CancellationToken,
) {
    let mut last_emit = Instant::now();
    let confirmations_before = handle.confirmations.load(Ordering::Relaxed);
    let interface_names = network.interface_names;
    let own_addresses = network.own_addresses;

    if let Some(discovery) = current_discovery(&handle) {
        // 广播本身要几秒钟，放到后台进行，不挡住已知地址探测与网段扫描。
        let announce = discovery.clone();
        tokio::spawn(async move {
            announce.announce().await;
        });

        let channels = known_channels(&handle);
        if !channels.is_empty() {
            // 已知地址探测是尽力而为：单个地址无响应属于正常情况，不中断扫描。
            if let Ok(found) = discovery.discover_known_http_channels(channels).await {
                for device in &found {
                    upsert_stateful(&handle, device);
                }
            }
        }
    }
    maybe_emit(&app, &handle, &shared, &mut last_emit, true);

    // 选定网段时无条件扫描；智能刷新只在没有任何确认时才回退扫本机全部网段。
    let subnets = match scope {
        ScanScope::Subnets(subnets) => subnets,
        ScanScope::Auto => {
            if handle.confirmations.load(Ordering::Relaxed) != confirmations_before {
                finish(&handle, &app, &shared, generation, &mut last_emit);
                return;
            }
            tokio::time::sleep(SMART_SCAN_GRACE).await;
            if cancel.is_cancelled()
                || handle.confirmations.load(Ordering::Relaxed) != confirmations_before
            {
                finish(&handle, &app, &shared, generation, &mut last_emit);
                return;
            }

            let rows = build_rows(&network.subnets, &interface_names, &own_addresses);
            if rows.is_empty() || !install_rows(&handle, generation, rows) {
                finish(&handle, &app, &shared, generation, &mut last_emit);
                return;
            }
            network.subnets
        }
    };

    let tasks = subnets.iter().enumerate().map(|(index, subnet)| {
        scan_subnet(
            &handle,
            &app,
            &shared,
            &client,
            &dto,
            &identity,
            &cancel,
            generation,
            index,
            &own_addresses,
            subnet.base,
            port,
        )
    });
    futures_util::future::join_all(tasks).await;

    finish(&handle, &app, &shared, generation, &mut last_emit);
}

#[allow(clippy::too_many_arguments)]
async fn scan_subnet(
    handle: &Arc<LanTransferHandle>,
    app: &AppHandle,
    shared: &Weak<SharedState>,
    client: &LsHttpClientV2,
    dto: &RegisterDtoV2,
    identity: &Arc<LanIdentity>,
    cancel: &CancellationToken,
    generation: u64,
    index: usize,
    own_addresses: &HashSet<Ipv4Addr>,
    base: Ipv4Addr,
    port: u16,
) {
    let mut last_emit = Instant::now();
    let hosts = subnet_hosts(base)
        .into_iter()
        .filter(|host| !own_addresses.contains(host));

    let probes = stream::iter(hosts).map(|host| {
        let own_fingerprint = identity.fingerprint.as_str();
        let cancel = cancel.clone();
        async move {
            probe_host(client, dto, own_fingerprint, host, port, &cancel)
                .await
                .map(|(body, fingerprint)| (host, body, fingerprint))
        }
    });
    pin_mut!(probes);
    let mut probes = probes.buffer_unordered(SCAN_CONCURRENCY);

    while let Some(found) = probes.next().await {
        let is_found = found.is_some();
        if let Some((host, body, fingerprint)) = found {
            handle.upsert_device(
                fingerprint,
                body.alias.clone(),
                body.device_model.clone(),
                body.device_type.as_ref().map(device_type_label),
                host.to_string(),
                port,
                ProtocolType::Https,
            );
        }
        if !record_host(handle, generation, index, is_found) {
            return;
        }
        maybe_emit(app, handle, shared, &mut last_emit, false);
    }

    mark_done(handle, generation, index);
    maybe_emit(app, handle, shared, &mut last_emit, true);
}

// 探测单台主机；HTTPS 下以对端证书指纹作为设备身份，失败与自身返回 None。
async fn probe_host(
    client: &LsHttpClientV2,
    dto: &RegisterDtoV2,
    own_fingerprint: &str,
    host: Ipv4Addr,
    port: u16,
    cancel: &CancellationToken,
) -> Option<(RegisterResponseDtoV2, String)> {
    if cancel.is_cancelled() {
        return None;
    }

    let host = host.to_string();
    let response = tokio::select! {
        _ = cancel.cancelled() => return None,
        result = client.register(ProtocolType::Https, &host, port, dto.clone()) => result.ok()?,
    };

    let fingerprint = response.cert_fingerprint?;
    if fingerprint == own_fingerprint {
        return None;
    }
    Some((response.body, fingerprint))
}

// 记录一次主机探测结果；返回 false 表示本次扫描已被更新的扫描取代。
fn record_host(
    handle: &Arc<LanTransferHandle>,
    generation: u64,
    index: usize,
    found: bool,
) -> bool {
    let mut inner = handle.inner.lock().unwrap();
    let Some(scan) = inner.scan.as_mut() else {
        return false;
    };
    if scan.generation != generation {
        return false;
    }
    let Some(row) = scan.subnets.get_mut(index) else {
        return false;
    };
    row.scanned += 1;
    if found {
        row.found += 1;
    }
    true
}

fn mark_done(handle: &Arc<LanTransferHandle>, generation: u64, index: usize) {
    let mut inner = handle.inner.lock().unwrap();
    let Some(scan) = inner.scan.as_mut() else {
        return;
    };
    if scan.generation != generation {
        return;
    }
    if let Some(row) = scan.subnets.get_mut(index) {
        row.done = true;
    }
}

// 智能刷新在确定要回退扫网段后，才把网段行填进状态。
fn install_rows(
    handle: &Arc<LanTransferHandle>,
    generation: u64,
    rows: Vec<ScanSubnetState>,
) -> bool {
    let mut inner = handle.inner.lock().unwrap();
    let Some(scan) = inner.scan.as_mut() else {
        return false;
    };
    if scan.generation != generation {
        return false;
    }
    scan.subnets = rows;
    true
}

fn finish(
    handle: &Arc<LanTransferHandle>,
    app: &AppHandle,
    shared: &Weak<SharedState>,
    generation: u64,
    last_emit: &mut Instant,
) {
    {
        let mut inner = handle.inner.lock().unwrap();
        if let Some(scan) = inner.scan.as_mut() {
            if scan.generation == generation {
                scan.running = false;
            }
        }
    }
    maybe_emit(app, handle, shared, last_emit, true);
}

fn maybe_emit(
    app: &AppHandle,
    handle: &Arc<LanTransferHandle>,
    shared: &Weak<SharedState>,
    last_emit: &mut Instant,
    force: bool,
) {
    if !force && last_emit.elapsed() < PROGRESS_EMIT_INTERVAL {
        return;
    }
    *last_emit = Instant::now();
    emit_state_with(app, handle, shared);
}

fn build_rows(
    subnets: &[ScanSubnet],
    interface_names: &HashMap<String, String>,
    own_addresses: &HashSet<Ipv4Addr>,
) -> Vec<ScanSubnetState> {
    subnets
        .iter()
        .map(|subnet| {
            let total = subnet_hosts(subnet.base)
                .into_iter()
                .filter(|host| !own_addresses.contains(host))
                .count() as u32;
            ScanSubnetState {
                cidr: subnet.cidr.clone(),
                interface: interface_names.get(&subnet.cidr).cloned().unwrap_or_default(),
                scanned: 0,
                total,
                found: 0,
                done: false,
            }
        })
        .collect()
}

// 已知设备列表：用记录过的地址再确认一次，适用于组播不达但路由可达的网络。
fn known_channels(handle: &LanTransferHandle) -> Vec<HttpChannel> {
    let inner = handle.inner.lock().unwrap();
    inner
        .devices
        .iter()
        .map(|device| HttpChannel {
            host: device.host.clone(),
            port: device.port,
            protocol: device.protocol,
        })
        .collect()
}

fn current_discovery(handle: &LanTransferHandle) -> Option<Arc<DiscoveryHandle>> {
    handle
        .inner
        .lock()
        .unwrap()
        .service
        .as_ref()
        .map(|service| service.discovery.clone())
}

fn upsert_stateful(handle: &Arc<LanTransferHandle>, device: &StatefulDevice) {
    let Some(channel) = device.device.channel.http() else {
        return;
    };
    handle.upsert_device(
        device.device.fingerprint.clone(),
        device.device.alias.clone(),
        device.device.device_model.clone(),
        device.device.device_type.as_ref().map(device_type_label),
        channel.host.clone(),
        channel.port,
        channel.protocol,
    );
}

// 本机的网段候选、网卡名与需要排除的本机地址。
fn local_network() -> Result<LocalNetwork, AppError> {
    let entries = local_subnets()?;
    let interface_names = entries
        .iter()
        .map(|entry| (entry.cidr.clone(), entry.interface.clone()))
        .collect::<HashMap<_, _>>();
    let subnets = entries
        .into_iter()
        .map(|entry| ScanSubnet {
            cidr: entry.cidr,
            base: entry.base,
        })
        .collect::<Vec<_>>();
    let own_addresses = local_ipv4_addresses()?;

    Ok(LocalNetwork {
        subnets,
        interface_names,
        own_addresses,
    })
}

// 列出本机非回环 IPv4 网卡所在的 /24 网段；同一网段只保留一项，真实网卡优先。
fn local_subnets() -> Result<Vec<LocalSubnet>, AppError> {
    let interfaces =
        if_addrs::get_if_addrs().map_err(|_| AppError::Message("lan_scan_no_interface".into()))?;
    let mut entries: Vec<LocalSubnet> = Vec::new();

    for interface in interfaces {
        if interface.is_loopback() {
            continue;
        }
        let IpAddr::V4(address) = interface.ip() else {
            continue;
        };
        let base = network_base(address);
        let cidr = cidr_of(base);
        let is_virtual = super::util::is_virtual_interface(&interface.name);

        match entries.iter_mut().find(|entry| entry.cidr == cidr) {
            // 同一网段出现在多张网卡上时保留真实网卡，虚拟网卡只在没有别的候选时使用。
            Some(existing) => {
                if existing.virtual_interface && !is_virtual {
                    existing.address = address;
                    existing.interface = interface.name.clone();
                    existing.virtual_interface = false;
                }
            }
            None => entries.push(LocalSubnet {
                cidr,
                base,
                address,
                interface: interface.name.clone(),
                virtual_interface: is_virtual,
            }),
        }
    }

    // 真实网卡在前，其余按网段排序，保证菜单顺序稳定。
    entries.sort_by(|left, right| {
        left.virtual_interface
            .cmp(&right.virtual_interface)
            .then_with(|| left.cidr.cmp(&right.cidr))
    });
    Ok(entries)
}

// 本机全部 IPv4 地址：扫描时跳过它们，避免把本机自己识别成对端设备。
fn local_ipv4_addresses() -> Result<HashSet<Ipv4Addr>, AppError> {
    let interfaces =
        if_addrs::get_if_addrs().map_err(|_| AppError::Message("lan_scan_no_interface".into()))?;
    Ok(interfaces
        .into_iter()
        .filter_map(|interface| match interface.ip() {
            IpAddr::V4(address) => Some(address),
            IpAddr::V6(_) => None,
        })
        .collect())
}

// 解析 `a.b.c.d/24`（前缀可省略）为该 /24 的网络地址。
fn parse_subnet(value: &str) -> Option<Ipv4Addr> {
    let (address, prefix) = match value.split_once('/') {
        Some((address, prefix)) => (address, Some(prefix)),
        None => (value, None),
    };
    if let Some(prefix) = prefix {
        if prefix.trim() != "24" {
            return None;
        }
    }
    let parsed: Ipv4Addr = address.trim().parse().ok()?;
    Some(network_base(parsed))
}

fn network_base(address: Ipv4Addr) -> Ipv4Addr {
    let octets = address.octets();
    Ipv4Addr::new(octets[0], octets[1], octets[2], 0)
}

fn cidr_of(base: Ipv4Addr) -> String {
    format!("{base}/24")
}

// 与上游传统发现一致：只扫 1..=254，跳过网络地址与广播地址。
fn subnet_hosts(base: Ipv4Addr) -> Vec<Ipv4Addr> {
    let octets = base.octets();
    (SUBNET_FIRST_HOST..=SUBNET_LAST_HOST)
        .map(|host| Ipv4Addr::new(octets[0], octets[1], octets[2], host))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{net::Ipv4Addr, sync::Arc, time::Duration};

    use localsend::{
        crypto::cert::generate_self_signed,
        http::{
            client::v2::LsHttpClientV2,
            dto_v2::RegisterDtoV2,
            server::{start_with_port, ServerConfigV2, TlsConfig},
            state::ClientInfo,
        },
        model::discovery::{DeviceType, PROTOCOL_VERSION_V2},
    };
    use tokio::sync::{mpsc, oneshot};
    use tokio_util::sync::CancellationToken;

    use super::{
        cidr_of, clear, mark_done, network_base, normalize_subnets, parse_subnet, probe_host,
        record_host, subnet_hosts, LanTransferHandle, ScanState, ScanSubnetState,
    };

    #[test]
    fn normalizes_subnet_input_to_its_network_address() {
        assert_eq!(
            parse_subnet("192.168.1.23"),
            Some(Ipv4Addr::new(192, 168, 1, 0))
        );
        assert_eq!(
            parse_subnet(" 192.168.1.23/24 "),
            Some(Ipv4Addr::new(192, 168, 1, 0))
        );
        assert_eq!(cidr_of(Ipv4Addr::new(10, 0, 5, 0)), "10.0.5.0/24");
        assert_eq!(
            network_base(Ipv4Addr::new(172, 16, 3, 9)),
            Ipv4Addr::new(172, 16, 3, 0)
        );
    }

    #[test]
    fn rejects_unsupported_or_invalid_subnets() {
        assert_eq!(parse_subnet("192.168.1.0/16"), None);
        assert_eq!(parse_subnet("192.168.1"), None);
        assert_eq!(parse_subnet("not-an-ip/24"), None);
        assert_eq!(parse_subnet(""), None);
    }

    #[test]
    fn deduplicates_requested_subnets() {
        let subnets = normalize_subnets(&[
            "192.168.1.10/24".to_string(),
            "192.168.1.30".to_string(),
            "10.0.0.5/24".to_string(),
        ])
        .expect("valid subnets");

        assert_eq!(subnets.len(), 2);
        assert_eq!(subnets[0].cidr, "192.168.1.0/24");
        assert_eq!(subnets[1].cidr, "10.0.0.0/24");
    }

    #[test]
    fn rejects_empty_and_invalid_requests() {
        assert!(normalize_subnets(&[]).is_err());
        assert!(normalize_subnets(&["192.168.1.0/30".to_string()]).is_err());
    }

    #[test]
    fn enumerates_the_254_hosts_of_a_subnet() {
        let hosts = subnet_hosts(Ipv4Addr::new(192, 168, 7, 0));

        assert_eq!(hosts.len(), 254);
        assert_eq!(hosts[0], Ipv4Addr::new(192, 168, 7, 1));
        assert_eq!(hosts[253], Ipv4Addr::new(192, 168, 7, 254));
    }

    // 进度只记在当前扫描上：被新一轮扫描取代后，旧任务不能再改写状态。
    #[test]
    fn records_progress_only_for_the_current_scan() {
        let handle = Arc::new(LanTransferHandle::new());
        let cancel = CancellationToken::new();
        handle.inner.lock().unwrap().scan = Some(ScanState {
            generation: 7,
            running: true,
            cancel: cancel.clone(),
            subnets: vec![ScanSubnetState {
                cidr: "192.168.1.0/24".into(),
                interface: "eth0".into(),
                scanned: 0,
                total: 254,
                found: 0,
                done: false,
            }],
        });

        assert!(record_host(&handle, 7, 0, true));
        assert!(record_host(&handle, 7, 0, false));
        assert!(!record_host(&handle, 8, 0, false));

        {
            let inner = handle.inner.lock().unwrap();
            let row = &inner.scan.as_ref().expect("scan").subnets[0];
            assert_eq!(row.scanned, 2);
            assert_eq!(row.found, 1);
            assert_eq!(row.total, 254);
        }

        mark_done(&handle, 8, 0);
        assert!(!handle.inner.lock().unwrap().scan.as_ref().unwrap().subnets[0].done);

        mark_done(&handle, 7, 0);
        assert!(handle.inner.lock().unwrap().scan.as_ref().unwrap().subnets[0].done);

        clear(&handle);
        assert!(cancel.is_cancelled());
        assert!(handle.inner.lock().unwrap().scan.is_none());
    }

    // 探测走真实 HTTPS 注册端点：验证能读到对端数据与证书指纹，并跳过自身指纹。
    #[tokio::test]
    async fn probes_a_real_https_server_and_skips_its_own_fingerprint() {
        let peer_cert = generate_self_signed().expect("peer cert");
        let own_cert = generate_self_signed().expect("own cert");
        let (event_tx, _event_rx) = mpsc::channel(16);
        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        let server = start_with_port(
            0,
            Some(TlsConfig {
                cert: peer_cert.certificate_pem.clone(),
                private_key: peer_cert.private_key_pem.clone(),
            }),
            ClientInfo {
                alias: "Peer".into(),
                version: PROTOCOL_VERSION_V2.into(),
                device_model: Some("Power Paste".into()),
                device_type: Some(DeviceType::Desktop),
                token: peer_cert.fingerprint.clone(),
            },
            None,
            Some(ServerConfigV2 {
                pin: None,
                verify_checksums: true,
                event_tx,
            }),
            None,
            stop_rx,
        )
        .await
        .expect("start server");

        let client = LsHttpClientV2::try_new(
            &own_cert.private_key_pem,
            &own_cert.certificate_pem,
            None,
            Some(Duration::from_millis(500)),
        )
        .expect("client");
        let dto = RegisterDtoV2 {
            alias: "Self".into(),
            version: PROTOCOL_VERSION_V2.into(),
            device_model: Some("Power Paste".into()),
            device_type: Some(DeviceType::Desktop),
            fingerprint: own_cert.fingerprint.clone(),
            port: 53317,
            protocol: localsend::model::discovery::ProtocolType::Https,
            download: false,
        };
        let cancel = CancellationToken::new();

        let found = probe_host(
            &client,
            &dto,
            &own_cert.fingerprint,
            Ipv4Addr::LOCALHOST,
            server.port(),
            &cancel,
        )
        .await
        .expect("peer must answer");
        assert_eq!(found.0.alias, "Peer");
        assert_eq!(found.1, peer_cert.fingerprint);

        // 对端指纹等于自身时说明扫到了本机，必须丢弃，避免把本机当成设备。
        let skipped = probe_host(
            &client,
            &dto,
            &peer_cert.fingerprint,
            Ipv4Addr::LOCALHOST,
            server.port(),
            &cancel,
        )
        .await;
        assert!(skipped.is_none());

        let cancelled = CancellationToken::new();
        cancelled.cancel();
        let stopped = probe_host(
            &client,
            &dto,
            &own_cert.fingerprint,
            Ipv4Addr::LOCALHOST,
            server.port(),
            &cancelled,
        )
        .await;
        assert!(stopped.is_none());

        let _ = stop_tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(3), server.wait_stopped()).await;
    }
}
