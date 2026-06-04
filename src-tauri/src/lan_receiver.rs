use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::{
    codecs::png::{CompressionType, FilterType, PngEncoder},
    imageops::FilterType as ResizeFilterType,
    ColorType, DynamicImage, GenericImageView, ImageEncoder,
};
use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use uuid::Uuid;

#[path = "lan_receiver_network.rs"]
mod lan_receiver_network;
#[path = "lan_receiver_page.rs"]
mod lan_receiver_page;
#[path = "lan_receiver_state.rs"]
mod lan_receiver_state;

use crate::{
    clipboard::write_item_to_clipboard_with_profile,
    history::{build_captured_clipboard, history_item_to_dto, store_capture_item},
    models::{
        AppError, CapturedClipboard, LanReceiverSession, LanReceiverStateDto, LanReceiverStatus,
        LanTransferFile, LanTransferMessage, SharedState, StoredClipboardItem,
        HISTORY_UPDATED_EVENT, LAN_RECEIVER_STATUS_EVENT,
    },
    paste_target::TargetProfile,
};
use lan_receiver_network::{build_qr_svg, local_lan_ips};
use lan_receiver_page::mobile_page;
use lan_receiver_state::{message_to_dto, now_ms, receiver_state_dto};

const UPLOAD_HARD_LIMIT: usize = 128 * 1024 * 1024;
const MAX_STORED_IMAGE_SIDE: u32 = 1600;
const MOBILE_POLL_MS: u64 = 1200;
const IDLE_SESSION_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const MAX_SESSION_MESSAGES: usize = 100;
const PHONE_SEEN_EMIT_INTERVAL: Duration = Duration::from_secs(5);
const MAX_IMAGE_WORKERS: usize = 2;

static ACTIVE_IMAGE_WORKERS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MobileClipboardPayload {
    kind: String,
    text: Option<String>,
    image_data: Option<String>,
}

pub(crate) fn get_state(state: &Arc<SharedState>) -> LanReceiverStateDto {
    cleanup_expired_session(state);
    let guard = state.lan_receiver.lock().unwrap();
    receiver_state_dto(guard.as_ref(), escape_url_component)
}

// 启动局域网互传会话，生成带随机令牌的手机访问地址和二维码。
pub(crate) fn start(
    app: AppHandle,
    state: Arc<SharedState>,
) -> Result<LanReceiverStateDto, AppError> {
    cleanup_expired_session(&state);
    if let Some(existing) = state.lan_receiver.lock().unwrap().as_ref() {
        return Ok(receiver_state_dto(Some(existing), escape_url_component));
    }

    let server = Server::http(("0.0.0.0", 0)).map_err(|error| anyhow::anyhow!("{error}"))?;
    let port = server
        .server_addr()
        .to_ip()
        .map(|addr| addr.port())
        .ok_or_else(|| anyhow::anyhow!("failed to resolve receiver port"))?;
    let ip_candidates = local_lan_ips().unwrap_or_else(|_| vec!["127.0.0.1".into()]);
    let ip = ip_candidates
        .first()
        .cloned()
        .unwrap_or_else(|| "127.0.0.1".into());
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
            ip_candidates,
            port,
            token: token.clone(),
            expires_at: Some(now + IDLE_SESSION_TIMEOUT),
            stop_requested: stop_requested.clone(),
            last_status: None,
            last_phone_seen: None,
            last_phone_seen_emit: None,
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
        cleanup_session_files(&session);
    }
    let dto = receiver_state_dto(None, escape_url_component);
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
    file_path: String,
    file_name: Option<String>,
    mime_type: Option<String>,
) -> Result<LanReceiverStateDto, AppError> {
    if state.lan_receiver.lock().unwrap().is_none() {
        return Err(AppError::Message("lan_transfer_not_running".into()));
    }

    let source_path = PathBuf::from(file_path);
    let metadata = fs::metadata(&source_path).map_err(anyhow::Error::from)?;
    if !metadata.is_file() {
        return Err(AppError::Message("lan_transfer_file_not_found".into()));
    }
    let size = usize::try_from(metadata.len())
        .map_err(|_| AppError::Message("request body too large".into()))?;
    if size == 0 {
        return Err(AppError::Message("empty_payload".into()));
    }
    if size > UPLOAD_HARD_LIMIT {
        return Err(AppError::Message("request body too large".into()));
    }

    let fallback_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("transfer-file")
        .to_string();
    let safe_name = sanitize_file_name(file_name.as_deref().unwrap_or(&fallback_name));
    let mime_type = mime_type
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| infer_mime_type(&source_path));
    let file_id = Uuid::new_v4().to_string();
    let kind = if mime_type.starts_with("image/") {
        "image"
    } else {
        "file"
    };
    let local_path = save_session_local_file_from_path(&state, &safe_name, &source_path)?;

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
                path: local_path.clone(),
                size,
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
            size: Some(size),
            image_data_url: None,
            download_url: Some(format!("/api/files/{file_id}")),
            local_path: Some(local_path),
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
            cleanup_session_files(&session);
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
            let dto = receiver_state_dto(None, escape_url_component);
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
            mobile_page(
                settings.max_image_bytes,
                &settings.accent_color,
                MOBILE_POLL_MS,
            ),
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

        let Some(permit) = ImageWorkerPermit::try_acquire() else {
            respond_error(request, &app, &state, anyhow::anyhow!("lan_transfer_busy"));
            return;
        };

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
            let _permit = permit;
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
                .map(|message| message_to_dto(message, token, escape_url_component))
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

    let body = match fs::File::open(&file.path) {
        Ok(body) => body,
        Err(_) => {
            respond_text(request, 404, "file not found");
            return;
        }
    };

    let mut response = Response::from_file(body).with_status_code(StatusCode(200));
    if let Ok(header) = Header::from_bytes("Content-Type", file.mime_type.as_str()) {
        response.add_header(header);
    }
    if let Ok(header) = Header::from_bytes("Content-Length", file.size.to_string().as_str()) {
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
    crate::sync::schedule_auto_sync(app.clone(), state.clone());
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
    let should_emit = {
        let mut guard = state.lan_receiver.lock().unwrap();
        let Some(session) = guard.as_mut() else {
            return;
        };
        let now = SystemTime::now();
        let was_connected = session
            .last_phone_seen
            .and_then(|seen| now.duration_since(seen).ok())
            .map(|elapsed| elapsed <= Duration::from_secs(15))
            .unwrap_or(false);
        let emit_interval_elapsed = session
            .last_phone_seen_emit
            .and_then(|emitted| now.duration_since(emitted).ok())
            .map(|elapsed| elapsed >= PHONE_SEEN_EMIT_INTERVAL)
            .unwrap_or(true);
        session.last_phone_seen = Some(now);
        if !was_connected || emit_interval_elapsed {
            session.last_phone_seen_emit = Some(now);
            true
        } else {
            false
        }
    };
    if should_emit {
        let dto = get_state(state);
        let _ = app.emit(LAN_RECEIVER_STATUS_EVENT, &dto);
    }
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
        prune_session_messages(session);
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

fn save_session_local_file_from_path(
    state: &Arc<SharedState>,
    file_name: &str,
    source_path: &Path,
) -> Result<PathBuf> {
    let root = state
        .paths
        .settings_path
        .parent()
        .context("settings parent missing")?
        .join("lan-transfer-sent");
    fs::create_dir_all(&root)?;
    let target = unique_file_path(&root, file_name);
    fs::copy(source_path, &target)?;
    Ok(target)
}

fn cleanup_session_files(session: &LanReceiverSession) {
    for file in session.files.values() {
        let _ = fs::remove_file(&file.path);
    }
}

fn message_file_id(message: &LanTransferMessage) -> Option<String> {
    message
        .download_url
        .as_ref()
        .and_then(|value| value.strip_prefix("/api/files/"))
        .map(ToString::to_string)
}

fn prune_session_messages(session: &mut LanReceiverSession) {
    if session.messages.len() <= MAX_SESSION_MESSAGES {
        return;
    }

    let removed_count = session.messages.len() - MAX_SESSION_MESSAGES;
    let removed = session.messages.drain(0..removed_count).collect::<Vec<_>>();
    for message in removed {
        if let Some(file_id) = message_file_id(&message) {
            if let Some(file) = session.files.remove(&file_id) {
                let _ = fs::remove_file(file.path);
            }
        }
    }
}

fn infer_mime_type(path: &Path) -> String {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "txt" | "log" | "md" => "text/plain",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .into()
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

struct ImageWorkerPermit;

impl ImageWorkerPermit {
    fn try_acquire() -> Option<Self> {
        let mut current = ACTIVE_IMAGE_WORKERS.load(Ordering::Relaxed);
        loop {
            if current >= MAX_IMAGE_WORKERS {
                return None;
            }
            match ACTIVE_IMAGE_WORKERS.compare_exchange(
                current,
                current + 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some(Self),
                Err(next) => current = next,
            }
        }
    }
}

impl Drop for ImageWorkerPermit {
    fn drop(&mut self) {
        ACTIVE_IMAGE_WORKERS.fetch_sub(1, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::{query_has_token, sanitize_file_name, split_target, unique_file_path};
    use crate::lan_receiver::lan_receiver_network::{
        extract_ipv4_candidates, lan_ipv4_score, usable_lan_ipv4,
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
        let candidates =
            crate::lan_receiver::lan_receiver_network::extract_windows_ipv4_candidates(text);
        assert_eq!(candidates, vec![Ipv4Addr::new(192, 168, 5, 174)]);
    }
}
