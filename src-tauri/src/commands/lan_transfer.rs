use std::{path::Path, sync::Arc};

use tauri::{AppHandle, State};

use crate::{
    lan_transfer::{
        inspect_selection, read_clipboard_selection, LanDecision, LanSelectionItemDto, LanStateDto,
        LanSubnetsDto,
    },
    models::{AppError, SharedState},
    system_open,
};

// 启动局域网互传服务：监听 LocalSend 默认端口并参与设备发现。
#[tauri::command]
pub(crate) async fn start_lan_transfer(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<LanStateDto, AppError> {
    let shared = state.inner().clone();
    shared.lan_transfer.start(&app, &shared).await
}

// 停止局域网互传服务并释放端口。
#[tauri::command]
pub(crate) async fn stop_lan_transfer(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<LanStateDto, AppError> {
    let shared = state.inner().clone();
    shared.lan_transfer.stop(&app, &shared).await
}

// 获取当前局域网互传状态，供前端渲染设备列表与传输进度。
#[tauri::command]
pub(crate) fn get_lan_transfer_state(
    state: State<'_, Arc<SharedState>>,
) -> Result<LanStateDto, AppError> {
    let settings = state.settings.lock().unwrap().clone();
    Ok(state.lan_transfer.state(&settings))
}

// 主动刷新一次设备发现。
#[tauri::command]
pub(crate) async fn refresh_lan_devices(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<LanStateDto, AppError> {
    let shared = state.inner().clone();
    shared.lan_transfer.refresh(&app, &shared).await
}

// 手动添加设备：向指定 IP/主机名发送注册请求，供组播不可用时使用。
#[tauri::command]
pub(crate) async fn add_lan_device(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    host: String,
    port: Option<u16>,
) -> Result<LanStateDto, AppError> {
    let shared = state.inner().clone();
    shared
        .lan_transfer
        .discover_address(&app, &shared, host, port.unwrap_or(0))
        .await
}

// 列出本机网卡所在的 /24 网段与上次选择，供刷新时挑网段扫描。
#[tauri::command]
pub(crate) fn list_lan_subnets(
    state: State<'_, Arc<SharedState>>,
) -> Result<LanSubnetsDto, AppError> {
    let last = state.settings.lock().unwrap().lan_scan_last_subnet.clone();
    crate::lan_transfer::list_subnets(last.as_deref())
}

// 只扫描指定网段：广播并探测已知地址后，逐台主机主动探测所选网段。
#[tauri::command]
pub(crate) async fn scan_lan_subnets(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    subnets: Vec<String>,
    port: Option<u16>,
) -> Result<LanStateDto, AppError> {
    let shared = state.inner().clone();
    shared
        .lan_transfer
        .scan_subnets(&app, &shared, subnets, port)
        .await
}

// 取消正在进行的网段扫描。
#[tauri::command]
pub(crate) fn cancel_lan_scan(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<LanStateDto, AppError> {
    let shared = state.inner().clone();
    Ok(shared.lan_transfer.cancel_scan(&app, &shared))
}

// 向指定设备发送文件，路径来自前端的文件选择器；pin 用于对端启用了 PIN 的场景。
#[tauri::command]
pub(crate) async fn send_lan_files(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    fingerprint: String,
    paths: Vec<String>,
    pin: Option<String>,
) -> Result<LanStateDto, AppError> {
    let shared = state.inner().clone();
    shared
        .lan_transfer
        .send_files(&app, &shared, fingerprint, paths, pin)
        .await
}

// 向指定设备发送文本，走协议原生文本消息；pin 用于对端启用了 PIN 的场景。
#[tauri::command]
pub(crate) async fn send_lan_text(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    fingerprint: String,
    text: String,
    pin: Option<String>,
) -> Result<LanStateDto, AppError> {
    let shared = state.inner().clone();
    shared
        .lan_transfer
        .send_text(&app, &shared, fingerprint, text, pin)
        .await
}

// 展开前端选择的文件与文件夹，返回统一的选择项元数据。
#[tauri::command]
pub(crate) async fn inspect_lan_selection(
    paths: Vec<String>,
) -> Result<Vec<LanSelectionItemDto>, AppError> {
    tauri::async_runtime::spawn_blocking(move || inspect_selection(paths))
        .await
        .map_err(|error| AppError::Message(error.to_string()))?
}

// 读取当前剪贴板内容，按文本、图片、文件列表的顺序返回可发送选择项。
#[tauri::command]
pub(crate) async fn read_lan_clipboard(
    app: AppHandle,
) -> Result<Vec<LanSelectionItemDto>, AppError> {
    tauri::async_runtime::spawn_blocking(move || read_clipboard_selection(&app))
        .await
        .map_err(|error| AppError::Message(error.to_string()))?
}

// 向指定设备发送统一的文件/文本选择项集合。
#[tauri::command]
pub(crate) async fn send_lan_items(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    fingerprint: String,
    items: Vec<LanSelectionItemDto>,
    pin: Option<String>,
) -> Result<LanStateDto, AppError> {
    let shared = state.inner().clone();
    shared
        .lan_transfer
        .send_items(&app, &shared, fingerprint, items, pin)
        .await
}

// 回应一次接收请求：accept / decline / accept-and-trust。
#[tauri::command]
pub(crate) fn respond_lan_request(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    request_id: String,
    decision: String,
) -> Result<LanStateDto, AppError> {
    let shared = state.inner().clone();
    let decision = match decision.as_str() {
        "accept" => LanDecision::Accept,
        "accept-and-trust" => LanDecision::AcceptAndTrust,
        _ => LanDecision::Decline,
    };
    shared
        .lan_transfer
        .respond(&app, &shared, &request_id, decision)
}

// 取消一次正在进行的发送。
#[tauri::command]
pub(crate) async fn cancel_lan_transfer(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    transfer_id: String,
) -> Result<LanStateDto, AppError> {
    let shared = state.inner().clone();
    shared
        .lan_transfer
        .cancel_transfer(&app, &shared, &transfer_id)
        .await
}

// 切换浏览器扫码页模式：none / share / receive。
#[tauri::command]
pub(crate) async fn set_lan_web_mode(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    mode: String,
    paths: Vec<String>,
) -> Result<LanStateDto, AppError> {
    let shared = state.inner().clone();
    shared
        .lan_transfer
        .set_web_mode(&app, &shared, &mode, paths)
        .await
}

// 打开已接收文件。
#[tauri::command]
pub(crate) fn open_lan_received_file(
    state: State<'_, Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    let path = state.lan_transfer.received_path(&id)?;
    open_received_path(&path)
}

// 在系统文件管理器中定位已接收文件。
#[tauri::command]
pub(crate) fn reveal_lan_received_file(
    state: State<'_, Arc<SharedState>>,
    id: String,
) -> Result<(), AppError> {
    let path = state.lan_transfer.received_path(&id)?;
    reveal_received_path(&path)
}

// 移除一个已信任设备。
#[tauri::command]
pub(crate) fn remove_lan_trusted_device(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    fingerprint: String,
) -> Result<(), AppError> {
    let shared = state.inner().clone();
    let mut settings = shared.settings.lock().unwrap().clone();
    settings
        .lan_trusted_devices
        .retain(|entry| entry.fingerprint != fingerprint);
    crate::storage::save_settings(&shared.paths, &settings)?;
    *shared.settings.lock().unwrap() = settings.clone();
    crate::lan_transfer::emit_state(&app, &shared.lan_transfer, &settings);
    Ok(())
}

fn open_received_path(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::Message("lan_transfer_file_not_found".into()));
    }
    system_open::open_path(path).map_err(AppError::from)
}

fn reveal_received_path(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::Message("lan_transfer_file_not_found".into()));
    }
    system_open::reveal_path(path).map_err(AppError::from)
}
