use std::{
    fs,
    io::{Cursor, Read},
    net::{Ipv4Addr, UdpSocket},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::{
    codecs::png::{CompressionType, FilterType, PngEncoder},
    imageops::FilterType as ResizeFilterType,
    ColorType, DynamicImage, GenericImageView, ImageEncoder,
};
use qrcode::{render::svg, QrCode};
use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use uuid::Uuid;

use crate::{
    clipboard::write_item_to_clipboard_with_profile,
    history::{build_captured_clipboard, history_item_to_dto, store_capture_item},
    models::{
        AppError, CapturedClipboard, LanReceiverSession, LanReceiverStateDto, LanReceiverStatus,
        LanTransferFile, LanTransferMessage, LanTransferMessageDto, SharedState,
        StoredClipboardItem, HISTORY_UPDATED_EVENT, LAN_RECEIVER_STATUS_EVENT,
    },
    paste_target::TargetProfile,
};

const UPLOAD_HARD_LIMIT: usize = 128 * 1024 * 1024;
const MAX_STORED_IMAGE_SIDE: u32 = 1600;
const MOBILE_POLL_MS: u64 = 1200;
const ACTIVE_DEVICE_WINDOW: Duration = Duration::from_secs(15);
const IDLE_SESSION_TIMEOUT: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MobileClipboardPayload {
    kind: String,
    text: Option<String>,
    image_data: Option<String>,
}

fn system_time_ms(value: SystemTime) -> Option<u64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}

fn now_ms() -> u64 {
    system_time_ms(SystemTime::now()).unwrap_or(0)
}

fn message_to_dto(message: &LanTransferMessage, token: &str) -> LanTransferMessageDto {
    let download_url = message.download_url.as_ref().map(|url| {
        if url.contains('?') {
            url.clone()
        } else {
            format!("{url}?token={}", escape_url_component(token))
        }
    });

    LanTransferMessageDto {
        id: message.id.clone(),
        sender: message.sender.clone(),
        kind: message.kind.clone(),
        text: message.text.clone(),
        file_name: message.file_name.clone(),
        mime_type: message.mime_type.clone(),
        size: message.size,
        image_data_url: message.image_data_url.clone(),
        download_url,
        has_local_file: message.local_path.is_some(),
        created_at: message.created_at,
        status: message.status.clone(),
    }
}

fn connected_devices(session: &LanReceiverSession) -> usize {
    session
        .last_phone_seen
        .and_then(|seen| seen.elapsed().ok())
        .map(|elapsed| usize::from(elapsed <= ACTIVE_DEVICE_WINDOW))
        .unwrap_or(0)
}

fn receiver_state_dto(session: Option<&LanReceiverSession>) -> LanReceiverStateDto {
    let Some(session) = session else {
        return LanReceiverStateDto {
            running: false,
            url: None,
            qr_svg: None,
            ip: None,
            port: None,
            token: None,
            expires_at: None,
            last_status: None,
            connected_devices: 0,
            messages: Vec::new(),
        };
    };

    LanReceiverStateDto {
        running: true,
        url: Some(session.url.clone()),
        qr_svg: Some(session.qr_svg.clone()),
        ip: Some(session.ip.clone()),
        port: Some(session.port),
        token: Some(session.token.clone()),
        expires_at: session.expires_at.and_then(system_time_ms),
        last_status: session.last_status.clone(),
        connected_devices: connected_devices(session),
        messages: session
            .messages
            .iter()
            .map(|message| message_to_dto(message, &session.token))
            .collect(),
    }
}

fn local_lan_ip() -> Result<String> {
    let mut candidates = local_ipv4_candidates();

    if let Ok(ip) = default_route_ipv4() {
        candidates.push(ip);
    }

    candidates
        .into_iter()
        .filter(|ip| usable_lan_ipv4(*ip))
        .max_by_key(|ip| lan_ipv4_score(*ip))
        .map(|ip| ip.to_string())
        .ok_or_else(|| anyhow::anyhow!("failed to resolve local lan ip"))
}

fn default_route_ipv4() -> Result<Ipv4Addr> {
    let socket = UdpSocket::bind(("0.0.0.0", 0))?;
    socket.connect(("8.8.8.8", 80))?;
    match socket.local_addr()?.ip() {
        std::net::IpAddr::V4(ip) => Ok(ip),
        std::net::IpAddr::V6(_) => anyhow::bail!("default route resolved to ipv6"),
    }
}

fn local_ipv4_candidates() -> Vec<Ipv4Addr> {
    platform_ipv4_candidates()
        .into_iter()
        .fold(Vec::new(), |mut candidates, ip| {
            if !candidates.contains(&ip) {
                candidates.push(ip);
            }
            candidates
        })
}

#[cfg(target_os = "windows")]
fn platform_ipv4_candidates() -> Vec<Ipv4Addr> {
    let output = match command_output_text("ipconfig", &["/all"]) {
        Some(output) => output,
        None => return Vec::new(),
    };
    extract_windows_ipv4_candidates(&output)
}

#[cfg(target_os = "macos")]
fn platform_ipv4_candidates() -> Vec<Ipv4Addr> {
    command_ipv4_candidates("ifconfig", &[])
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_ipv4_candidates() -> Vec<Ipv4Addr> {
    let candidates = command_ipv4_candidates("ip", &["-4", "addr"]);
    if candidates.is_empty() {
        command_ipv4_candidates("ifconfig", &[])
    } else {
        candidates
    }
}

#[cfg(not(any(windows, target_os = "macos", unix)))]
fn platform_ipv4_candidates() -> Vec<Ipv4Addr> {
    Vec::new()
}

#[cfg(not(target_os = "windows"))]
fn command_ipv4_candidates(program: &str, args: &[&str]) -> Vec<Ipv4Addr> {
    command_output_text(program, args)
        .map(|text| extract_ipv4_candidates(&text))
        .unwrap_or_default()
}

fn command_output_text(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

fn extract_ipv4_candidates(text: &str) -> Vec<Ipv4Addr> {
    let mut candidates = Vec::new();
    for token in text.split(|value: char| !(value.is_ascii_digit() || value == '.')) {
        let Ok(ip) = token.parse::<Ipv4Addr>() else {
            continue;
        };
        if !candidates.contains(&ip) {
            candidates.push(ip);
        }
    }
    candidates
}

#[cfg(target_os = "windows")]
fn extract_windows_ipv4_candidates(text: &str) -> Vec<Ipv4Addr> {
    let mut candidates = Vec::new();
    for line in text.lines().filter(|line| line.contains("IPv4")) {
        for ip in extract_ipv4_candidates(line) {
            if !candidates.contains(&ip) {
                candidates.push(ip);
            }
        }
    }
    candidates
}

fn usable_lan_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    !ip.is_loopback()
        && !ip.is_link_local()
        && !ip.is_multicast()
        && !ip.is_broadcast()
        && !ip.is_unspecified()
        && !(octets[0] == 198 && (octets[1] == 18 || octets[1] == 19))
        && octets[3] != 0
        && octets[3] != 255
}

fn lan_ipv4_score(ip: Ipv4Addr) -> i32 {
    let octets = ip.octets();
    let host_score = match octets[3] {
        1 => -50,
        2..=9 => -10,
        _ => i32::from(octets[3]).min(40),
    };
    if octets[0] == 192 && octets[1] == 168 {
        return 400 + host_score;
    }
    if octets[0] == 10 {
        return 390 + host_score;
    }
    if octets[0] == 172 && (16..=31).contains(&octets[1]) {
        return 380 + host_score;
    }
    if octets[0] == 100 && (64..=127).contains(&octets[1]) {
        return 120 + host_score;
    }
    80 + host_score
}

fn build_qr_svg(url: &str) -> Result<String> {
    let code = QrCode::new(url.as_bytes())?;
    Ok(code
        .render::<svg::Color<'_>>()
        .min_dimensions(220, 220)
        .dark_color(svg::Color("#1d232d"))
        .light_color(svg::Color("#ffffff"))
        .build())
}

pub(crate) fn get_state(state: &Arc<SharedState>) -> LanReceiverStateDto {
    cleanup_expired_session(state);
    let guard = state.lan_receiver.lock().unwrap();
    receiver_state_dto(guard.as_ref())
}

// 启动局域网互传会话，生成带随机令牌的手机访问地址和二维码。
pub(crate) fn start(
    app: AppHandle,
    state: Arc<SharedState>,
) -> Result<LanReceiverStateDto, AppError> {
    cleanup_expired_session(&state);
    if let Some(existing) = state.lan_receiver.lock().unwrap().as_ref() {
        return Ok(receiver_state_dto(Some(existing)));
    }

    let server = Server::http(("0.0.0.0", 0)).map_err(|error| anyhow::anyhow!("{error}"))?;
    let port = server
        .server_addr()
        .to_ip()
        .map(|addr| addr.port())
        .ok_or_else(|| anyhow::anyhow!("failed to resolve receiver port"))?;
    let ip = local_lan_ip().unwrap_or_else(|_| "127.0.0.1".into());
    let token = Uuid::new_v4().to_string();
    let url = format!("http://{ip}:{port}/?token={token}");
    let qr_svg = build_qr_svg(&url).map_err(anyhow::Error::from)?;
    let stop_requested = Arc::new(AtomicBool::new(false));

    {
        let mut guard = state.lan_receiver.lock().unwrap();
        let now = SystemTime::now();
        *guard = Some(LanReceiverSession {
            url,
            qr_svg,
            ip,
            port,
            token: token.clone(),
            expires_at: Some(now + IDLE_SESSION_TIMEOUT),
            stop_requested: stop_requested.clone(),
            last_status: None,
            last_phone_seen: None,
            last_activity: now,
            messages: Vec::new(),
            files: std::collections::HashMap::new(),
        });
    }

    let server_app = app.clone();
    let server_state = state.clone();
    thread::spawn(move || run_server(server_app, server_state, server, token, stop_requested));

    let dto = get_state(&state);
    let _ = app.emit(LAN_RECEIVER_STATUS_EVENT, &dto);
    Ok(dto)
}

fn session_token_matches(state: &Arc<SharedState>, token: &str) -> bool {
    state
        .lan_receiver
        .lock()
        .unwrap()
        .as_ref()
        .map(|session| session.token == token)
        .unwrap_or(false)
}

// 停止当前局域网互传会话，并让已生成的二维码立即失效。
pub(crate) fn stop(
    app: AppHandle,
    state: Arc<SharedState>,
) -> Result<LanReceiverStateDto, AppError> {
    if let Some(session) = state.lan_receiver.lock().unwrap().take() {
        session.stop_requested.store(true, Ordering::Relaxed);
    }
    let dto = receiver_state_dto(None);
    let _ = app.emit(LAN_RECEIVER_STATUS_EVENT, &dto);
    Ok(dto)
}

// 电脑端发送文字给手机，消息会出现在手机聊天页。
pub(crate) fn send_desktop_text(
    app: AppHandle,
    state: Arc<SharedState>,
    text: String,
) -> Result<LanReceiverStateDto, AppError> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err(AppError::Message("empty_payload".into()));
    }
    push_message(
        &app,
        &state,
        LanTransferMessage {
            id: Uuid::new_v4().to_string(),
            sender: "desktop".into(),
            kind: "text".into(),
            text: Some(text),
            file_name: None,
            mime_type: None,
            size: None,
            image_data_url: None,
            download_url: None,
            local_path: None,
            created_at: now_ms(),
            status: "sent".into(),
        },
    )
}

// 电脑端发送文件或图片给手机，手机端通过消息中的下载链接获取。
pub(crate) fn send_desktop_file(
    app: AppHandle,
    state: Arc<SharedState>,
    file_name: String,
    mime_type: Option<String>,
    bytes: Vec<u8>,
) -> Result<LanReceiverStateDto, AppError> {
    if bytes.is_empty() {
        return Err(AppError::Message("empty_payload".into()));
    }
    if bytes.len() > UPLOAD_HARD_LIMIT {
        return Err(AppError::Message("request body too large".into()));
    }

    let safe_name = sanitize_file_name(&file_name);
    let mime_type = mime_type
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "application/octet-stream".into());
    let file_id = Uuid::new_v4().to_string();
    let kind = if mime_type.starts_with("image/") {
        "image"
    } else {
        "file"
    };
    let image_data_url = if kind == "image" {
        Some(format!("data:{mime_type};base64,{}", BASE64.encode(&bytes)))
    } else {
        None
    };
    let local_path = save_session_local_file(&state, &safe_name, &bytes).ok();

    {
        let mut guard = state.lan_receiver.lock().unwrap();
        let session = guard
            .as_mut()
            .ok_or_else(|| AppError::Message("lan_transfer_not_running".into()))?;
        session.files.insert(
            file_id.clone(),
            LanTransferFile {
                file_name: safe_name.clone(),
                mime_type: mime_type.clone(),
                bytes: bytes.clone(),
            },
        );
    }

    push_message(
        &app,
        &state,
        LanTransferMessage {
            id: Uuid::new_v4().to_string(),
            sender: "desktop".into(),
            kind: kind.into(),
            text: None,
            file_name: Some(safe_name),
            mime_type: Some(mime_type),
            size: Some(bytes.len()),
            image_data_url,
            download_url: Some(format!("/api/files/{file_id}")),
            local_path,
            created_at: now_ms(),
            status: "sent".into(),
        },
    )
}

fn cleanup_expired_session(state: &Arc<SharedState>) -> bool {
    let mut guard = state.lan_receiver.lock().unwrap();
    let expired = guard
        .as_ref()
        .map(|session| {
            session
                .expires_at
                .map(|expires_at| expires_at <= SystemTime::now())
                .unwrap_or(false)
        })
        .unwrap_or(false);
    if expired {
        if let Some(session) = guard.take() {
            session.stop_requested.store(true, Ordering::Relaxed);
        }
        return true;
    }
    false
}

fn run_server(
    app: AppHandle,
    state: Arc<SharedState>,
    server: Server,
    token: String,
    stop_requested: Arc<AtomicBool>,
) {
    while !stop_requested.load(Ordering::Relaxed) {
        if cleanup_expired_session(&state) {
            let dto = receiver_state_dto(None);
            let _ = app.emit(LAN_RECEIVER_STATUS_EVENT, dto);
        }
        if !session_token_matches(&state, &token) {
            break;
        }

        match server.recv_timeout(Duration::from_millis(120)) {
            Ok(Some(request)) => {
                let worker_app = app.clone();
                let worker_state = state.clone();
                let worker_token = token.clone();
                thread::spawn(move || {
                    handle_request(worker_app, worker_state, request, &worker_token);
                });
            }
            Ok(None) => {}
            Err(error) => {
                set_status(
                    &app,
                    &state,
                    LanReceiverStatus {
                        kind: "error".into(),
                        message: format!("listener_error: {error}"),
                        received_kind: None,
                    },
                );
                break;
            }
        }
    }
}

fn handle_request(app: AppHandle, state: Arc<SharedState>, request: Request, token: &str) {
    route_request(app, state, request, token);
}

fn route_request(app: AppHandle, state: Arc<SharedState>, mut request: Request, token: &str) {
    let (path, query) = split_target(request.url());
    if request.method() == &Method::Get && path == "/" {
        if !query_has_token(&query, token) {
            respond_text(request, 403, "invalid token");
            return;
        }
        mark_phone_seen(&app, &state);
        let settings = state.settings.lock().unwrap().clone();
        respond_html(
            request,
            mobile_page(settings.max_image_bytes, &settings.accent_color),
        );
        return;
    }

    if request.method() == &Method::Get && path == "/app-icon.png" {
        respond_png(request, include_bytes!("../icons/32x32.png").to_vec());
        return;
    }

    if request.method() == &Method::Get && path == "/api/messages" {
        if !query_has_token(&query, token) {
            respond_json(request, 403, r#"{"ok":false,"message":"invalid_token"}"#);
            return;
        }
        mark_phone_seen(&app, &state);
        let body = mobile_messages_json(&state, token);
        respond_json(request, 200, &body);
        return;
    }

    if request.method() == &Method::Get && path.starts_with("/api/files/") {
        if !query_has_token(&query, token) {
            respond_text(request, 403, "invalid token");
            return;
        }
        mark_phone_seen(&app, &state);
        let file_id = path.trim_start_matches("/api/files/");
        respond_session_file(request, &state, file_id);
        return;
    }

    if request.method() == &Method::Post && path == "/api/clipboard" {
        if !query_has_token(&query, token) {
            respond_json(request, 403, r#"{"ok":false,"message":"invalid_token"}"#);
            return;
        }
        mark_phone_seen(&app, &state);

        let body = match read_tiny_request_body(&mut request, UPLOAD_HARD_LIMIT) {
            Ok(body) => body,
            Err(error) => {
                respond_error(request, &app, &state, error);
                return;
            }
        };
        match receive_payload(app.clone(), state.clone(), &body) {
            Ok(kind) => respond_json(
                request,
                200,
                &format!(r#"{{"ok":true,"kind":"{}"}}"#, escape_json(&kind)),
            ),
            Err(error) => respond_error(request, &app, &state, error),
        }
        return;
    }

    if request.method() == &Method::Post && path == "/api/clipboard/text" {
        if !query_has_token(&query, token) {
            respond_json(request, 403, r#"{"ok":false,"message":"invalid_token"}"#);
            return;
        }
        mark_phone_seen(&app, &state);

        let body = match read_tiny_request_body(&mut request, UPLOAD_HARD_LIMIT) {
            Ok(body) => body,
            Err(error) => {
                respond_error(request, &app, &state, error);
                return;
            }
        };
        match receive_text_payload(app.clone(), state.clone(), &body) {
            Ok(kind) => respond_json(
                request,
                200,
                &format!(r#"{{"ok":true,"kind":"{}"}}"#, escape_json(&kind)),
            ),
            Err(error) => respond_error(request, &app, &state, error),
        }
        return;
    }

    if request.method() == &Method::Post && path == "/api/clipboard/image" {
        if !query_has_token(&query, token) {
            respond_json(request, 403, r#"{"ok":false,"message":"invalid_token"}"#);
            return;
        }
        mark_phone_seen(&app, &state);

        let body = match read_tiny_request_body(&mut request, UPLOAD_HARD_LIMIT) {
            Ok(body) => body,
            Err(error) => {
                respond_error(request, &app, &state, error);
                return;
            }
        };
        if body.is_empty() {
            respond_error(request, &app, &state, anyhow::anyhow!("empty_payload"));
            return;
        }
        let max_image_bytes = state.settings.lock().unwrap().max_image_bytes;
        if body.len() > max_image_bytes {
            respond_error(request, &app, &state, anyhow::anyhow!("image_too_large"));
            return;
        }

        set_status(
            &app,
            &state,
            LanReceiverStatus {
                kind: "processing".into(),
                message: "processing_image".into(),
                received_kind: Some("image".into()),
            },
        );
        let worker_app = app.clone();
        let worker_state = state.clone();
        let image_bytes = body;
        let message_id = query_param(&query, "clientId").map(|value| sanitize_message_id(&value));
        thread::spawn(move || {
            if let Err(error) = receive_image_payload(
                worker_app.clone(),
                worker_state.clone(),
                &image_bytes,
                message_id,
            ) {
                set_status(
                    &worker_app,
                    &worker_state,
                    LanReceiverStatus {
                        kind: "error".into(),
                        message: error.to_string(),
                        received_kind: None,
                    },
                );
            }
        });

        respond_json(
            request,
            202,
            r#"{"ok":true,"kind":"image","status":"processing"}"#,
        );
        return;
    }

    if request.method() == &Method::Post && path == "/api/clipboard/file" {
        if !query_has_token(&query, token) {
            respond_json(request, 403, r#"{"ok":false,"message":"invalid_token"}"#);
            return;
        }
        mark_phone_seen(&app, &state);

        let body = match read_tiny_request_body(&mut request, UPLOAD_HARD_LIMIT) {
            Ok(body) => body,
            Err(error) => {
                respond_error(request, &app, &state, error);
                return;
            }
        };
        let file_name = query_param(&query, "name").unwrap_or_else(|| "transfer-file".into());
        let mime_type =
            query_param(&query, "mime").unwrap_or_else(|| "application/octet-stream".into());
        let message_id = query_param(&query, "clientId").map(|value| sanitize_message_id(&value));
        match receive_file_payload(
            app.clone(),
            state.clone(),
            &file_name,
            &mime_type,
            &body,
            message_id,
        ) {
            Ok(kind) => respond_json(
                request,
                200,
                &format!(r#"{{"ok":true,"kind":"{}"}}"#, escape_json(&kind)),
            ),
            Err(error) => respond_error(request, &app, &state, error),
        }
        return;
    }

    respond_text(request, 404, "not found");
}

fn mobile_messages_json(state: &Arc<SharedState>, token: &str) -> String {
    cleanup_expired_session(state);
    let guard = state.lan_receiver.lock().unwrap();
    let messages = guard
        .as_ref()
        .map(|session| {
            session
                .messages
                .iter()
                .map(|message| message_to_dto(message, token))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    serde_json::json!({
        "ok": true,
        "messages": messages,
        "pollMs": MOBILE_POLL_MS
    })
    .to_string()
}

fn respond_session_file(request: Request, state: &Arc<SharedState>, file_id: &str) {
    let file = {
        let guard = state.lan_receiver.lock().unwrap();
        guard
            .as_ref()
            .and_then(|session| session.files.get(file_id).cloned())
    };

    let Some(file) = file else {
        respond_text(request, 404, "file not found");
        return;
    };

    let mut response = Response::from_data(file.bytes).with_status_code(StatusCode(200));
    if let Ok(header) = Header::from_bytes("Content-Type", file.mime_type.as_str()) {
        response.add_header(header);
    }
    if let Ok(header) = Header::from_bytes(
        "Content-Disposition",
        format!(
            "attachment; filename=\"{}\"",
            escape_header_value(&file.file_name)
        ),
    ) {
        response.add_header(header);
    }
    if let Ok(header) = Header::from_bytes("Cache-Control", "no-store") {
        response.add_header(header);
    }
    let _ = request.respond(response);
}

fn read_tiny_request_body(request: &mut Request, max_body: usize) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    request
        .as_reader()
        .take((max_body + 1) as u64)
        .read_to_end(&mut body)?;
    if body.len() > max_body {
        anyhow::bail!("request body too large");
    }
    Ok(body)
}

fn receive_payload(app: AppHandle, state: Arc<SharedState>, body: &[u8]) -> Result<String> {
    let payload: MobileClipboardPayload = serde_json::from_slice(body)?;
    let text = payload.text.unwrap_or_default();
    let text = text.trim().to_string();
    let image_data = payload
        .image_data
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let has_text = !text.is_empty();
    let has_image = image_data.is_some();

    if has_text && has_image {
        anyhow::bail!("text_and_image_are_mutually_exclusive");
    }
    if !has_text && !has_image {
        anyhow::bail!("empty_payload");
    }

    let settings = state.settings.lock().unwrap().clone();
    let capture = if payload.kind == "text" && has_text {
        build_captured_clipboard(&settings, text, None, None, None, None, None, None, None)?
    } else if payload.kind == "image" && has_image {
        let image_data = image_data.unwrap();
        let raw = image_data
            .split_once(',')
            .map(|(_, value)| value)
            .unwrap_or(image_data.as_str());
        let bytes = BASE64.decode(raw)?;
        if bytes.len() > settings.max_image_bytes {
            anyhow::bail!("image_too_large");
        }
        let original_mime = detect_image_mime(&bytes).map(ToString::to_string);
        let decoded = image::load_from_memory(&bytes)?;
        let (width, height) = decoded.dimensions();
        let png_bytes = encode_png(decoded)?;
        if png_bytes.len() > settings.max_image_bytes {
            anyhow::bail!("image_too_large");
        }
        build_captured_clipboard(
            &settings,
            String::new(),
            None,
            None,
            Some(png_bytes),
            Some(bytes),
            original_mime,
            Some(width),
            Some(height),
        )?
    } else {
        anyhow::bail!("invalid_payload_kind");
    }
    .context("unsupported_payload")?;

    store_and_write_capture(app, state, &settings, capture, None)
}

fn receive_text_payload(app: AppHandle, state: Arc<SharedState>, body: &[u8]) -> Result<String> {
    let text = String::from_utf8(body.to_vec())?.trim().to_string();
    receive_capture(app, state, None, |settings| {
        build_captured_clipboard(settings, text, None, None, None, None, None, None, None)
    })
}

fn receive_image_payload(
    app: AppHandle,
    state: Arc<SharedState>,
    body: &[u8],
    message_id: Option<String>,
) -> Result<String> {
    if body.is_empty() {
        anyhow::bail!("empty_payload");
    }

    receive_capture(app, state, message_id, |settings| {
        if body.len() > settings.max_image_bytes {
            anyhow::bail!("image_too_large");
        }
        let decoded = image::load_from_memory(body)?;
        let (width, height) = decoded.dimensions();
        let png_bytes = encode_png_for_storage(decoded, settings.max_image_bytes)?;
        let original_mime = detect_image_mime(body).map(ToString::to_string);
        if png_bytes.len() > settings.max_image_bytes {
            anyhow::bail!("image_too_large");
        }
        let image_hash = crate::storage::image_hash_from_png_bytes(&png_bytes)?;
        Ok(Some(CapturedClipboard::Image {
            hash: image_hash,
            preview: format!("Image {width}x{height}"),
            png_bytes,
            original_bytes: Some(body.to_vec()),
            original_mime,
            image_width: width,
            image_height: height,
        }))
    })
}

fn receive_file_payload(
    app: AppHandle,
    state: Arc<SharedState>,
    file_name: &str,
    mime_type: &str,
    body: &[u8],
    message_id: Option<String>,
) -> Result<String> {
    if body.is_empty() {
        anyhow::bail!("empty_payload");
    }

    let safe_name = sanitize_file_name(file_name);
    let target_path = save_uploaded_file(&app, &state, &safe_name, body)?;
    push_message(
        &app,
        &state,
        LanTransferMessage {
            id: message_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            sender: "phone".into(),
            kind: "file".into(),
            text: Some(target_path.to_string_lossy().to_string()),
            file_name: Some(safe_name),
            mime_type: Some(mime_type.to_string()),
            size: Some(body.len()),
            image_data_url: None,
            download_url: None,
            local_path: Some(target_path.clone()),
            created_at: now_ms(),
            status: "saved".into(),
        },
    )?;
    set_status(
        &app,
        &state,
        LanReceiverStatus {
            kind: "success".into(),
            message: "received_file".into(),
            received_kind: Some("file".into()),
        },
    );
    Ok("file".into())
}

fn detect_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.starts_with(b"BM") {
        return Some("image/bmp");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

fn receive_capture<F>(
    app: AppHandle,
    state: Arc<SharedState>,
    message_id: Option<String>,
    build: F,
) -> Result<String>
where
    F: FnOnce(&crate::models::AppSettings) -> Result<Option<CapturedClipboard>>,
{
    let settings = state.settings.lock().unwrap().clone();
    let capture = build(&settings)?.context("unsupported_payload")?;
    store_and_write_capture(app, state, &settings, capture, message_id)
}

fn store_and_write_capture(
    app: AppHandle,
    state: Arc<SharedState>,
    settings: &crate::models::AppSettings,
    capture: CapturedClipboard,
    message_id: Option<String>,
) -> Result<String> {
    let received_kind = capture_kind(&capture).to_string();
    let image_data_url = capture_image_data_url(&capture);
    let text = capture_text(&capture);
    let item = {
        let mut store = state.history_store.lock().unwrap();
        store_capture_item(&mut store, capture, Some(("Mobile".into(), None)), settings)?
    };

    let _ = app.emit(HISTORY_UPDATED_EVENT, history_item_to_dto(&item));
    crate::capture::mark_clipboard_suppressed(&state, item.hash.clone());
    write_received_item_to_clipboard(&app, &item)?;
    push_message(
        &app,
        &state,
        LanTransferMessage {
            id: message_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            sender: "phone".into(),
            kind: received_kind.clone(),
            text,
            file_name: None,
            mime_type: if received_kind == "image" {
                Some("image/png".into())
            } else {
                None
            },
            size: item.image_display_byte_size(),
            image_data_url,
            download_url: None,
            local_path: None,
            created_at: now_ms(),
            status: "copied".into(),
        },
    )?;
    set_status(
        &app,
        &state,
        LanReceiverStatus {
            kind: "success".into(),
            message: "received".into(),
            received_kind: Some(received_kind.clone()),
        },
    );
    Ok(received_kind)
}

fn mark_phone_seen(app: &AppHandle, state: &Arc<SharedState>) {
    {
        let mut guard = state.lan_receiver.lock().unwrap();
        if let Some(session) = guard.as_mut() {
            session.last_phone_seen = Some(SystemTime::now());
        }
    }
    let dto = get_state(state);
    let _ = app.emit(LAN_RECEIVER_STATUS_EVENT, &dto);
}

fn capture_text(capture: &CapturedClipboard) -> Option<String> {
    match capture {
        CapturedClipboard::Text { text, .. }
        | CapturedClipboard::Link { text, .. }
        | CapturedClipboard::Mixed { text, .. } => {
            Some(text.clone()).filter(|value| !value.is_empty())
        }
        CapturedClipboard::Image { .. } => None,
    }
}

fn capture_image_data_url(capture: &CapturedClipboard) -> Option<String> {
    match capture {
        CapturedClipboard::Image {
            original_bytes,
            original_mime,
            png_bytes,
            ..
        } => {
            if let Some(bytes) = original_bytes.as_ref().filter(|bytes| !bytes.is_empty()) {
                let mime = original_mime
                    .as_deref()
                    .filter(|value| value.starts_with("image/"))
                    .unwrap_or("image/png");
                return Some(format!("data:{mime};base64,{}", BASE64.encode(bytes)));
            }
            Some(format!(
                "data:image/png;base64,{}",
                BASE64.encode(png_bytes)
            ))
        }
        CapturedClipboard::Mixed { png_bytes, .. } => png_bytes
            .as_ref()
            .map(|bytes| format!("data:image/png;base64,{}", BASE64.encode(bytes))),
        _ => None,
    }
}

fn write_received_item_to_clipboard(app: &AppHandle, item: &StoredClipboardItem) -> Result<()> {
    #[cfg(windows)]
    {
        if item.kind == "image" {
            let png_bytes = item.image_png.as_deref().context("image payload missing")?;
            crate::clipboard::write_image_to_clipboard(png_bytes)?;
            return Ok(());
        }
    }

    write_item_to_clipboard_with_profile(app, item, TargetProfile::Generic).map(|_| ())
}

fn push_message(
    app: &AppHandle,
    state: &Arc<SharedState>,
    message: LanTransferMessage,
) -> Result<LanReceiverStateDto, AppError> {
    {
        let mut guard = state.lan_receiver.lock().unwrap();
        let session = guard
            .as_mut()
            .ok_or_else(|| AppError::Message("lan_transfer_not_running".into()))?;
        let now = SystemTime::now();
        session.last_activity = now;
        session.expires_at = Some(now + IDLE_SESSION_TIMEOUT);
        session.messages.push(message);
    }
    let dto = get_state(state);
    let _ = app.emit(LAN_RECEIVER_STATUS_EVENT, &dto);
    Ok(dto)
}

fn resolve_download_dir(app: &AppHandle, state: &Arc<SharedState>) -> Result<PathBuf> {
    let configured = state
        .settings
        .lock()
        .unwrap()
        .lan_transfer_download_dir
        .clone();
    let path = if let Some(path) = configured {
        PathBuf::from(path)
    } else {
        app.path().download_dir()?
    };
    validate_download_dir(&path)?;
    Ok(path)
}

pub(crate) fn validate_download_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("lan_transfer_download_dir_missing");
    }
    if !path.is_dir() {
        anyhow::bail!("lan_transfer_download_dir_not_directory");
    }
    let probe = path.join(format!(".power-paste-write-test-{}", Uuid::new_v4()));
    fs::write(&probe, b"ok").context("lan_transfer_download_dir_not_writable")?;
    fs::remove_file(&probe).context("lan_transfer_download_dir_cleanup_failed")?;
    Ok(())
}

fn save_uploaded_file(
    app: &AppHandle,
    state: &Arc<SharedState>,
    file_name: &str,
    body: &[u8],
) -> Result<PathBuf> {
    let dir = resolve_download_dir(app, state)?;
    let target = unique_file_path(&dir, file_name);
    let mut cursor = Cursor::new(body);
    let mut file = fs::File::create(&target)?;
    std::io::copy(&mut cursor, &mut file)?;
    Ok(target)
}

fn save_session_local_file(
    state: &Arc<SharedState>,
    file_name: &str,
    body: &[u8],
) -> Result<PathBuf> {
    let root = state
        .paths
        .settings_path
        .parent()
        .context("settings parent missing")?
        .join("lan-transfer-sent");
    fs::create_dir_all(&root)?;
    let target = unique_file_path(&root, file_name);
    fs::write(&target, body)?;
    Ok(target)
}

pub(crate) fn message_local_path(state: &Arc<SharedState>, id: &str) -> Result<PathBuf> {
    let guard = state.lan_receiver.lock().unwrap();
    let path = guard
        .as_ref()
        .and_then(|session| session.messages.iter().find(|message| message.id == id))
        .and_then(|message| message.local_path.clone())
        .context("lan_transfer_file_not_found")?;
    Ok(path)
}

fn sanitize_file_name(value: &str) -> String {
    let name = value
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("transfer-file")
        .trim();
    let sanitized = name
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .trim()
        .to_string();

    if sanitized.is_empty() {
        "transfer-file".into()
    } else {
        sanitized.chars().take(180).collect()
    }
}

fn sanitize_message_id(value: &str) -> String {
    let sanitized = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();

    if sanitized.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        sanitized.chars().take(96).collect()
    }
}

fn unique_file_path(dir: &Path, file_name: &str) -> PathBuf {
    let candidate = dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }

    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("transfer-file");
    let extension = path.extension().and_then(|value| value.to_str());

    for index in 1..1000 {
        let next_name = match extension {
            Some(extension) if !extension.is_empty() => format!("{stem} ({index}).{extension}"),
            _ => format!("{stem} ({index})"),
        };
        let next = dir.join(next_name);
        if !next.exists() {
            return next;
        }
    }

    dir.join(format!("{stem}-{}", Uuid::new_v4()))
}

fn encode_png(image: DynamicImage) -> Result<Vec<u8>> {
    encode_png_bytes(image)
}

fn encode_png_for_storage(image: DynamicImage, max_bytes: usize) -> Result<Vec<u8>> {
    let (width, height) = image.dimensions();
    let longest_side = width.max(height);
    if longest_side <= MAX_STORED_IMAGE_SIDE {
        let png_bytes = encode_png_bytes(image.clone())?;
        if png_bytes.len() <= max_bytes {
            return Ok(png_bytes);
        }
    }

    let scale = MAX_STORED_IMAGE_SIDE as f32 / longest_side.max(1) as f32;
    let next_width = ((width as f32 * scale).round() as u32).max(1);
    let next_height = ((height as f32 * scale).round() as u32).max(1);
    let resized = image.resize(next_width, next_height, ResizeFilterType::Triangle);
    let resized_png = encode_png_bytes(resized)?;
    if resized_png.len() <= max_bytes {
        return Ok(resized_png);
    }

    anyhow::bail!("image_too_large_after_png_conversion")
}

fn encode_png_bytes(image: DynamicImage) -> Result<Vec<u8>> {
    let rgba = image.to_rgba8();
    let mut bytes = Vec::new();
    let encoder =
        PngEncoder::new_with_quality(&mut bytes, CompressionType::Fast, FilterType::NoFilter);
    encoder.write_image(
        rgba.as_raw(),
        rgba.width(),
        rgba.height(),
        ColorType::Rgba8.into(),
    )?;
    Ok(bytes)
}

fn capture_kind(capture: &CapturedClipboard) -> &'static str {
    match capture {
        CapturedClipboard::Text { .. } => "text",
        CapturedClipboard::Link { .. } => "link",
        CapturedClipboard::Image { .. } => "image",
        CapturedClipboard::Mixed { .. } => "mixed",
    }
}

fn set_status(app: &AppHandle, state: &Arc<SharedState>, status: LanReceiverStatus) {
    {
        let mut guard = state.lan_receiver.lock().unwrap();
        if let Some(session) = guard.as_mut() {
            session.last_status = Some(status);
        }
    }
    let dto = get_state(state);
    let _ = app.emit(LAN_RECEIVER_STATUS_EVENT, dto);
}

fn split_target(target: &str) -> (String, String) {
    target
        .split_once('?')
        .map(|(path, query)| (path.to_string(), query.to_string()))
        .unwrap_or_else(|| (target.to_string(), String::new()))
}

fn query_has_token(query: &str, expected: &str) -> bool {
    query
        .split('&')
        .filter_map(|part| part.split_once('='))
        .any(|(key, value)| key == "token" && value == expected)
}

fn query_param(query: &str, key: &str) -> Option<String> {
    query
        .split('&')
        .filter_map(|part| part.split_once('='))
        .find(|(name, _)| *name == key)
        .map(|(_, value)| percent_decode(value))
}

fn percent_decode(value: &str) -> String {
    let mut bytes = Vec::new();
    let raw = value.as_bytes();
    let mut index = 0;
    while index < raw.len() {
        if raw[index] == b'%' && index + 2 < raw.len() {
            if let Ok(hex) = u8::from_str_radix(&value[index + 1..index + 3], 16) {
                bytes.push(hex);
                index += 3;
                continue;
            }
        }
        bytes.push(if raw[index] == b'+' { b' ' } else { raw[index] });
        index += 1;
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn respond_html(request: Request, body: String) {
    respond(request, 200, "text/html; charset=utf-8", body);
}

fn respond_json(request: Request, status: u16, body: &str) {
    respond(
        request,
        status,
        "application/json; charset=utf-8",
        body.to_string(),
    );
}

fn respond_text(request: Request, status: u16, body: &str) {
    respond(
        request,
        status,
        "text/plain; charset=utf-8",
        body.to_string(),
    );
}

fn respond_png(request: Request, body: Vec<u8>) {
    let mut response = Response::from_data(body).with_status_code(StatusCode(200));
    if let Ok(header) = Header::from_bytes("Content-Type", "image/png") {
        response.add_header(header);
    }
    if let Ok(header) = Header::from_bytes("Cache-Control", "no-store") {
        response.add_header(header);
    }
    let _ = request.respond(response);
}

fn respond_error(
    request: Request,
    app: &AppHandle,
    state: &Arc<SharedState>,
    error: anyhow::Error,
) {
    set_status(
        app,
        state,
        LanReceiverStatus {
            kind: "error".into(),
            message: error.to_string(),
            received_kind: None,
        },
    );
    respond_json(
        request,
        400,
        &format!(
            r#"{{"ok":false,"message":"{}"}}"#,
            escape_json(&error.to_string())
        ),
    );
}

fn respond(request: Request, status: u16, content_type: &'static str, body: String) {
    let mut response = Response::from_string(body).with_status_code(StatusCode(status));
    if let Ok(header) = Header::from_bytes("Content-Type", content_type) {
        response.add_header(header);
    }
    if let Ok(header) = Header::from_bytes("Cache-Control", "no-store") {
        response.add_header(header);
    }
    let _ = request.respond(response);
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn escape_header_value(value: &str) -> String {
    value.replace('\\', "_").replace('"', "_")
}

fn escape_url_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn mobile_accent_palette(accent_color: &str) -> (&'static str, &'static str, &'static str) {
    match accent_color {
        "ocean" => ("#68b6ff", "#3e7fe6", "#0d1a2a"),
        "jade" => ("#62d6b1", "#2f9f83", "#0c1f1b"),
        "rose" => ("#f08db0", "#d45a86", "#2b1019"),
        _ => ("#f0b35f", "#dd8648", "#24160d"),
    }
}

fn mobile_page(max_image_bytes: usize, accent_color: &str) -> String {
    let (accent_primary, accent_strong, accent_text) = mobile_accent_palette(accent_color);
    r#"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Power Paste Transfer</title>
  <style>
    :root { color-scheme: light dark; font-family: ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
    * { box-sizing: border-box; }
    body { margin: 0; min-height: 100vh; background: #f5f1e9; color: #1c232b; --accent-primary: __ACCENT_PRIMARY__; --accent-strong: __ACCENT_STRONG__; --accent-text: __ACCENT_TEXT__; }
    main { min-height: 100vh; display: grid; grid-template-rows: auto 1fr auto; max-width: 720px; margin: 0 auto; }
    header { padding: 14px 16px 8px; border-bottom: 1px solid rgba(42, 48, 56, .12); }
    h1 { margin: 0; font-size: 18px; line-height: 1.2; }
    .status-row { display: flex; align-items: center; gap: 7px; margin-top: 4px; min-height: 20px; color: #65707d; font-size: 13px; }
    .status-dot { width: 8px; height: 8px; flex: 0 0 auto; border-radius: 999px; }
    .status-dot.connected { background: #44d17f; box-shadow: 0 0 12px rgba(68, 209, 127, .86); }
    .status-dot.disconnected { background: #f55656; box-shadow: 0 0 12px rgba(245, 86, 86, .86); }
    .status { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .messages { padding: 14px 12px 128px; overflow: auto; }
    .message { display: grid; grid-template-columns: 34px minmax(0, 1fr); gap: 8px; align-items: end; margin: 0 0 10px; }
    .message.phone { grid-template-columns: minmax(0, 1fr) 34px; }
    .message.phone .avatar { grid-column: 2; }
    .message.phone .bubble { grid-column: 1; grid-row: 1; justify-self: end; }
    .avatar { width: 34px; height: 34px; display: grid; place-items: center; border-radius: 0; background: transparent; color: #65707d; overflow: hidden; }
    .avatar img { width: 24px; height: 24px; }
    .avatar svg { width: 20px; height: 20px; }
    .bubble { max-width: min(86%, 520px); border-radius: 16px; padding: 10px 12px; background: #fff; box-shadow: 0 8px 24px rgba(32, 38, 46, .08); word-break: break-word; }
    .phone .bubble { background: #1f2937; color: #fff; }
    .bubble img { display: block; max-width: 100%; border-radius: 10px; margin-top: 6px; }
    .file-row { display: flex; gap: 10px; align-items: center; }
    .file-icon { width: 34px; height: 34px; border-radius: 10px; display: grid; place-items: center; background: color-mix(in srgb, var(--accent-strong) 16%, transparent); color: var(--accent-strong); font-weight: 800; }
    .file-name { font-weight: 700; }
    .file-size { font-size: 12px; opacity: .68; }
    .upload-progress { display: grid; gap: 5px; margin-top: 8px; min-width: 180px; font-size: 12px; opacity: .78; }
    .upload-progress progress { width: 100%; height: 5px; overflow: hidden; border: 0; border-radius: 999px; background: color-mix(in srgb, currentColor 18%, transparent); }
    .upload-progress progress::-webkit-progress-bar { background: color-mix(in srgb, currentColor 18%, transparent); }
    .upload-progress progress::-webkit-progress-value { border-radius: 999px; background: currentColor; }
    .upload-progress progress::-moz-progress-bar { border-radius: 999px; background: currentColor; }
    .upload-error { margin-top: 6px; font-size: 12px; opacity: .82; }
    a { display: inline-flex; margin-top: 8px; color: inherit; font-weight: 700; }
    form { position: fixed; left: 50%; bottom: max(14px, env(safe-area-inset-bottom)); transform: translateX(-50%); width: min(696px, calc(100% - 24px)); display: grid; grid-template-columns: 42px 1fr 42px; gap: 8px; padding: 10px 12px; background: rgba(245, 241, 233, .94); backdrop-filter: blur(18px); border: 1px solid rgba(42, 48, 56, .12); border-radius: 18px; box-shadow: 0 14px 40px rgba(32,38,46,.16); }
    textarea { min-height: 42px; max-height: 120px; resize: none; border: 1px solid rgba(42, 48, 56, .18); border-radius: 13px; padding: 10px 12px; font: inherit; background: #fff; color: inherit; }
    button.icon { width: 42px; height: 42px; border: 0; border-radius: 13px; background: linear-gradient(135deg, var(--accent-strong), var(--accent-primary)); color: var(--accent-text); font-size: 20px; font-weight: 800; }
    button.icon.secondary { background: #e7ded1; color: #242b35; }
    input[type="file"] { display: none; }
    @media (prefers-color-scheme: dark) {
      body { background: #11161d; color: #f5efe7; }
      header, form { border-color: rgba(255,255,255,.1); }
      form { background: rgba(17, 22, 29, .92); }
      textarea, .bubble { background: #1d242d; }
      .phone .bubble { background: var(--accent-strong); color: var(--accent-text); }
      .avatar { background: transparent; color: #aab3bf; }
      button.icon.secondary { background: #2b3440; color: #f5efe7; }
      .status-row { color: #aab3bf; }
    }
  </style>
</head>
<body>
  <main>
    <header>
      <h1>Power Paste 互传</h1>
      <div class="status-row">
        <span id="statusDot" class="status-dot disconnected" aria-hidden="true"></span>
        <div id="status" class="status">正在连接...</div>
      </div>
    </header>
    <section id="messages" class="messages" aria-live="polite"></section>
    <form id="composer">
      <button id="fileButton" class="icon secondary" type="button" aria-label="选择文件">+</button>
      <textarea id="textInput" rows="1" placeholder="输入消息"></textarea>
      <button id="sendButton" class="icon" type="submit" aria-label="发送">↑</button>
      <input id="fileInput" type="file" multiple />
    </form>
  </main>
  <script>
    const token = new URLSearchParams(location.search).get('token') || '';
    const statusDot = document.getElementById('statusDot');
    const statusEl = document.getElementById('status');
    const messagesEl = document.getElementById('messages');
    const composer = document.getElementById('composer');
    const textInput = document.getElementById('textInput');
    const fileInput = document.getElementById('fileInput');
    const fileButton = document.getElementById('fileButton');
    const maxImageBytes = __MAX_IMAGE_BYTES__;
    const maxTransferFiles = 9;
    let seen = new Set();
    let messageRows = new Map();
    let pollTimer = null;
    let wasConnected = false;
    let disconnectAlertShown = false;

    function setConnectionState(connected, message) {
      statusDot.className = 'status-dot ' + (connected ? 'connected' : 'disconnected');
      statusEl.textContent = message;
      if (connected) {
        wasConnected = true;
        disconnectAlertShown = false;
      } else if (wasConnected && !disconnectAlertShown) {
        disconnectAlertShown = true;
        window.alert('连接已断开，请退出页面后重新扫码连接。');
      }
    }

    function bytesLabel(size) {
      if (!size) return '';
      if (size < 1000) return size + ' B';
      if (size < 1000000) return Math.round(size / 1000) + ' KB';
      return (size / 1000000).toFixed(1) + ' MB';
    }

    function renderMessage(message) {
      const existing = messageRows.get(message.id);
      if (existing && existing.dataset.localUpload !== 'true') return;
      if (existing) existing.remove();
      seen.add(message.id);
      const row = document.createElement('article');
      if (message.localUpload) row.dataset.localUpload = 'true';
      row.className = 'message ' + message.sender;
      const avatar = document.createElement('div');
      avatar.className = 'avatar';
      if (message.sender === 'desktop') {
        const icon = document.createElement('img');
        icon.alt = '';
        icon.src = '/app-icon.png';
        avatar.appendChild(icon);
      } else {
        avatar.innerHTML = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 3h8a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2Zm2 15h4" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/></svg>';
      }
      const bubble = document.createElement('div');
      bubble.className = 'bubble';

      if (message.kind === 'text' || message.kind === 'link' || message.kind === 'mixed') {
        const text = document.createElement('div');
        text.textContent = message.text || '';
        bubble.appendChild(text);
      }

      if (message.imageDataUrl) {
        const image = document.createElement('img');
        image.alt = message.fileName || 'image';
        image.src = message.imageDataUrl;
        bubble.appendChild(image);
      }

      if (message.kind === 'file' || message.downloadUrl || message.fileName) {
        const wrap = document.createElement('div');
        wrap.className = 'file-row';
        const icon = document.createElement('span');
        icon.className = 'file-icon';
        icon.textContent = 'F';
        const info = document.createElement('div');
        const name = document.createElement('div');
        name.className = 'file-name';
        name.textContent = message.fileName || '文件';
        const size = document.createElement('div');
        size.className = 'file-size';
        size.textContent = bytesLabel(message.size);
        info.append(name, size);
        wrap.append(icon, info);
        bubble.appendChild(wrap);
      }

      if (message.downloadUrl) {
        const link = document.createElement('a');
        link.href = message.downloadUrl;
        link.download = message.fileName || '';
        link.textContent = '下载';
        bubble.appendChild(link);
      }

      if (message.status === 'uploading' || message.status === 'processing') {
        const progressWrap = document.createElement('div');
        progressWrap.className = 'upload-progress';
        const progressText = document.createElement('span');
        const progress = Math.max(0, Math.min(100, Number(message.progress || 0)));
        progressText.textContent = message.status === 'processing' ? '已发送，正在处理...' : '发送中 ' + progress + '%';
        progressWrap.appendChild(progressText);
        if (message.status === 'uploading') {
          const progressBar = document.createElement('progress');
          progressBar.max = 100;
          progressBar.value = progress;
          progressWrap.appendChild(progressBar);
        }
        bubble.appendChild(progressWrap);
      }

      if (message.status === 'failed') {
        const error = document.createElement('div');
        error.className = 'upload-error';
        error.textContent = message.text || '发送失败';
        bubble.appendChild(error);
      }

      row.append(avatar, bubble);
      messagesEl.appendChild(row);
      messageRows.set(message.id, row);
      messagesEl.scrollTop = messagesEl.scrollHeight;
    }

    async function pollMessages() {
      try {
        const response = await fetch('/api/messages?token=' + encodeURIComponent(token), { cache: 'no-store' });
        const result = await response.json();
        if (!result.ok) throw new Error(result.message || '连接失败');
        for (const message of result.messages || []) renderMessage(message);
        setConnectionState(true, '已连接');
        pollTimer = window.setTimeout(pollMessages, result.pollMs || __POLL_MS__);
      } catch (error) {
        setConnectionState(false, error.message || String(error));
        pollTimer = window.setTimeout(pollMessages, 2500);
      }
    }

    async function sendText(text) {
      const response = await fetch('/api/clipboard/text?token=' + encodeURIComponent(token), {
        method: 'POST',
        headers: { 'Content-Type': 'text/plain; charset=utf-8' },
        body: text
      });
      const result = await response.json();
      if (!result.ok) throw new Error(result.message || '发送失败');
    }

    function sendFile(file, messageId, onProgress) {
      if (file.type.startsWith('image/') && file.size > maxImageBytes) {
        return Promise.reject(new Error('图片超过电脑端大小限制'));
      }
      return new Promise((resolve, reject) => {
        const endpoint = file.type.startsWith('image/') ? '/api/clipboard/image' : '/api/clipboard/file';
        const params = new URLSearchParams({
          token,
          name: file.name || 'file',
          mime: file.type || 'application/octet-stream',
          clientId: messageId
        });
        const xhr = new XMLHttpRequest();
        xhr.open('POST', endpoint + '?' + params.toString());
        xhr.setRequestHeader('Content-Type', file.type || 'application/octet-stream');
        xhr.upload.onprogress = (event) => {
          if (event.lengthComputable) {
            onProgress(Math.max(1, Math.min(99, Math.round((event.loaded / event.total) * 100))));
          }
        };
        xhr.onload = () => {
          let result = null;
          try {
            result = JSON.parse(xhr.responseText || '{}');
          } catch {
            reject(new Error('发送失败'));
            return;
          }
          if (xhr.status < 200 || xhr.status >= 300 || !result.ok) {
            reject(new Error(result.message || '发送失败'));
            return;
          }
          onProgress(100);
          resolve(result);
        };
        xhr.onerror = () => reject(new Error('网络连接失败'));
        xhr.send(file);
      });
    }

    composer.addEventListener('submit', async (event) => {
      event.preventDefault();
      const text = textInput.value.trim();
      if (!text) return;
      textInput.value = '';
      statusEl.textContent = '发送中...';
      try {
        await sendText(text);
        await pollMessages();
      } catch (error) {
        statusEl.textContent = error.message || String(error);
      }
    });

    fileButton.addEventListener('click', () => fileInput.click());
    fileInput.addEventListener('change', async () => {
      const selectedFiles = Array.from(fileInput.files || []);
      if (!selectedFiles.length) return;
      fileInput.value = '';
      const files = selectedFiles.slice(0, maxTransferFiles);
      statusEl.textContent = selectedFiles.length > maxTransferFiles
        ? '一次最多选择 9 个文件或图片，已发送前 9 个'
        : '上传中...';

      for (const [index, file] of files.entries()) {
        const messageId = 'phone-upload-' + Date.now() + '-' + index;
        const previewUrl = file.type.startsWith('image/') ? URL.createObjectURL(file) : null;
        const baseMessage = {
          id: messageId,
          sender: 'phone',
          kind: file.type.startsWith('image/') ? 'image' : 'file',
          fileName: file.name || 'file',
          mimeType: file.type || 'application/octet-stream',
          size: file.size,
          imageDataUrl: previewUrl,
          progress: 0,
          status: 'uploading',
          localUpload: true
        };
        renderMessage(baseMessage);
        sendFile(file, messageId, (progress) => {
          renderMessage({ ...baseMessage, progress, status: 'uploading' });
        }).then(() => {
          renderMessage({ ...baseMessage, progress: 100, status: 'processing' });
          statusEl.textContent = '已发送，等待电脑处理...';
          pollMessages();
        }).catch((error) => {
          renderMessage({ ...baseMessage, progress: 100, status: 'failed', text: error.message || String(error) });
          statusEl.textContent = error.message || String(error);
        });
      }
    });

    window.addEventListener('beforeunload', () => {
      if (pollTimer) window.clearTimeout(pollTimer);
    });
    pollMessages();
  </script>
</body>
</html>"#
    .replace("__MAX_IMAGE_BYTES__", &max_image_bytes.to_string())
    .replace("__POLL_MS__", &MOBILE_POLL_MS.to_string())
    .replace("__ACCENT_PRIMARY__", accent_primary)
    .replace("__ACCENT_STRONG__", accent_strong)
    .replace("__ACCENT_TEXT__", accent_text)
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::{
        extract_ipv4_candidates, lan_ipv4_score, query_has_token, sanitize_file_name, split_target,
        unique_file_path, usable_lan_ipv4,
    };

    #[test]
    fn validates_token_query_pair() {
        assert!(query_has_token("token=abc&x=1", "abc"));
        assert!(!query_has_token("token=abc", "def"));
    }

    #[test]
    fn splits_path_and_query() {
        let (path, query) = split_target("/api/clipboard?token=abc");
        assert_eq!(path, "/api/clipboard");
        assert_eq!(query, "token=abc");
    }

    #[test]
    fn sanitizes_file_names() {
        assert_eq!(sanitize_file_name("../a:b?.txt"), "a_b_.txt");
        assert_eq!(sanitize_file_name("..."), "transfer-file");
    }

    #[test]
    fn keeps_unique_file_name_when_available() {
        let dir = std::env::temp_dir();
        let path = unique_file_path(&dir, "power-paste-unique-name-test.txt");
        assert!(path.ends_with("power-paste-unique-name-test.txt"));
    }

    #[test]
    fn excludes_benchmark_network_from_lan_candidates() {
        assert!(!usable_lan_ipv4(Ipv4Addr::new(198, 18, 0, 1)));
        assert!(!usable_lan_ipv4(Ipv4Addr::new(198, 19, 0, 1)));
        assert!(usable_lan_ipv4(Ipv4Addr::new(192, 168, 5, 174)));
    }

    #[test]
    fn prefers_private_home_lan_address() {
        let home_lan = Ipv4Addr::new(192, 168, 5, 174);
        let carrier_nat = Ipv4Addr::new(100, 64, 1, 2);
        assert!(lan_ipv4_score(home_lan) > lan_ipv4_score(carrier_nat));
    }

    #[test]
    fn prefers_host_address_over_common_gateway_address() {
        let host_ip = Ipv4Addr::new(192, 168, 5, 174);
        let gateway_ip = Ipv4Addr::new(192, 168, 5, 1);
        assert!(lan_ipv4_score(host_ip) > lan_ipv4_score(gateway_ip));
    }

    #[test]
    fn extracts_ipv4_addresses_from_command_output() {
        let text = "IPv4 Address . . . . . . . . . . . : 192.168.5.174\nMask 255.255.255.0";
        let candidates = extract_ipv4_candidates(text);
        assert!(candidates.contains(&Ipv4Addr::new(192, 168, 5, 174)));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn extracts_only_windows_ipv4_address_lines() {
        let text = "\
IPv4 Address. . . . . . . . . . . : 192.168.5.174(Preferred)
Subnet Mask . . . . . . . . . . . : 255.255.255.0
Default Gateway . . . . . . . . . : 192.168.5.1";
        let candidates = super::extract_windows_ipv4_candidates(text);
        assert_eq!(candidates, vec![Ipv4Addr::new(192, 168, 5, 174)]);
    }
}
