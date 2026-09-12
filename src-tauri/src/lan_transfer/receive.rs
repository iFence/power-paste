//! 接收链路：处理协议服务端事件，把收到的文本、图片与文件落到剪贴板、历史与保存目录。

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Weak,
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use bytes::Bytes;
use image::{
    codecs::png::{CompressionType, FilterType, PngEncoder},
    imageops::FilterType as ResizeFilterType,
    ColorType, DynamicImage, GenericImageView, ImageEncoder,
};
use localsend::{
    http::server::{
        common::save::FileUploadTarget,
        v2::{PrepareUploadDecisionV2, ServerEventV2, SessionEndReasonV2},
    },
    model::transfer::FileDto,
};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot};

use super::{
    confirm_device, emit_state_with, is_text_package, is_trusted, text_package, util, LanDecision,
    LanTransferHandle, TransferEntry, INCOMING_TIMEOUT, TEXT_RESTORE_MAX_BYTES,
};
use crate::{
    clipboard::write_item_to_clipboard_with_profile,
    history::{build_captured_clipboard, history_item_to_dto, store_capture_item},
    models::{SharedState, HISTORY_UPDATED_EVENT},
    paste_target::TargetProfile,
};

// 单张图片存入历史时的最长边与体积上限。
const MAX_STORED_IMAGE_SIDE: u32 = 1600;

// 一次接收会话的上下文，用于把 upload 事件映射回传输记录。
pub(super) struct ReceiveSession {
    pub(super) transfer_id: String,
    pub(super) files: HashMap<String, FileDto>,
}

// 处理协议服务端事件流，直到服务停止。
pub(crate) fn spawn_server_events(
    app: AppHandle,
    handle: Arc<LanTransferHandle>,
    shared: Weak<SharedState>,
    mut rx: mpsc::Receiver<ServerEventV2>,
) {
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                ServerEventV2::Register { ip, info } => {
                    confirm_device(&handle, &info, ip.to_string(), info.fingerprint.clone());
                    emit_state_with(&app, &handle, &shared);
                }
                ServerEventV2::PrepareUpload {
                    session_id,
                    ip,
                    info,
                    cert_fingerprint,
                    files,
                    decision_tx,
                } => {
                    handle_prepare_upload(
                        &app,
                        &handle,
                        &shared,
                        session_id,
                        ip.to_string(),
                        info,
                        cert_fingerprint,
                        files,
                        decision_tx,
                    );
                }
                ServerEventV2::FileUpload {
                    session_id,
                    file_id,
                    file,
                    target_tx,
                } => {
                    handle_file_upload(
                        &app, &handle, &shared, session_id, file_id, file, target_tx,
                    );
                }
                ServerEventV2::SessionEnd { session_id, reason } => {
                    finish_session(&app, &handle, &shared, &session_id, reason);
                }
                ServerEventV2::PrepareUploadAborted { session_id } => {
                    clear_incoming(&handle, &session_id, true);
                    emit_state_with(&app, &handle, &shared);
                }
                ServerEventV2::CancelReceived { ip, session_id } => {
                    super::send::handle_peer_cancel(&handle, &ip.to_string(), &session_id);
                }
                ServerEventV2::ListenerFailed { error } => {
                    handle.inner.lock().unwrap().warning = Some("lan_listener_failed".into());
                    emit_state_with(&app, &handle, &shared);
                    // 监听套接字已失效，服务自身不再接受连接：通知自愈流程重建服务。
                    handle.schedule_recovery(&app, &shared, error);
                }
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn handle_prepare_upload(
    app: &AppHandle,
    handle: &Arc<LanTransferHandle>,
    shared: &Weak<SharedState>,
    session_id: String,
    ip: String,
    info: localsend::http::dto_v2::RegisterDtoV2,
    cert_fingerprint: Option<String>,
    files: HashMap<String, FileDto>,
    decision_tx: oneshot::Sender<PrepareUploadDecisionV2>,
) {
    let fingerprint = cert_fingerprint.unwrap_or_else(|| info.fingerprint.clone());
    confirm_device(handle, &info, ip.clone(), fingerprint.clone());

    let Some(shared) = shared.upgrade() else {
        let _ = decision_tx.send(PrepareUploadDecisionV2::Decline);
        return;
    };
    let settings = shared.settings.lock().unwrap().clone();

    // 协议原生文本消息：无需传输文件，文本就在 preview 里。
    let (text_messages, file_ids) = text_package::split_text_messages(&files);
    let text_message = text_package::joined_text_message(&text_messages);
    let trusted = is_trusted(&settings, &fingerprint) || settings.lan_receive_policy == "auto";

    // 受信设备（或自动接受）：立刻接受；文本消息只需 204，文件才建立接收会话。
    if trusted {
        let accepted = file_ids.clone();
        if decision_tx
            .send(PrepareUploadDecisionV2::Accept(accepted))
            .is_ok()
        {
            for (_, text) in &text_messages {
                record_received_text(app, &shared, &settings, text);
                record_text_transfer(handle, &info.alias, text);
            }
            if !file_ids.is_empty() {
                let received_files = files
                    .iter()
                    .filter(|(id, _)| file_ids.contains(*id))
                    .map(|(id, file)| (id.clone(), file.clone()))
                    .collect::<HashMap<_, _>>();
                if !received_files.is_empty() {
                    start_receive_session(handle, &session_id, &info.alias, &received_files);
                }
            }
        }
        emit_state_with(app, handle, &Arc::downgrade(&shared));
        return;
    }

    // 未受信设备：文本消息同样需要用户确认，弹窗里直接展示消息正文。
    let request_id = uuid::Uuid::new_v4().to_string();
    let total_bytes = files
        .iter()
        .filter(|(id, _)| file_ids.contains(*id))
        .map(|(_, file)| file.size)
        .sum();
    let (decision_local_tx, decision_local_rx) = oneshot::channel::<LanDecision>();
    {
        let mut inner = handle.inner.lock().unwrap();
        inner.incoming = Some(super::IncomingEntry {
            request_id: request_id.clone(),
            session_id: session_id.clone(),
            alias: info.alias.clone(),
            device_model: info.device_model.clone(),
            ip: ip.clone(),
            fingerprint: fingerprint.clone(),
            files: match &text_message {
                Some(_) => files
                    .iter()
                    .filter(|(id, _)| file_ids.contains(*id))
                    .map(|(_, file)| (file.file_name.clone(), file.size))
                    .collect(),
                None => files
                    .values()
                    .map(|file| (file.file_name.clone(), file.size))
                    .collect(),
            },
            total_bytes,
            text_message: text_message.clone(),
        });
        inner.pending.insert(request_id.clone(), decision_local_tx);
    }
    emit_state_with(app, handle, &Arc::downgrade(&shared));

    let app = app.clone();
    let handle = handle.clone();
    let shared = Arc::downgrade(&shared);
    tokio::spawn(async move {
        let decision = tokio::select! {
            decision = decision_local_rx => decision.unwrap_or(LanDecision::Decline),
            _ = tokio::time::sleep(INCOMING_TIMEOUT) => LanDecision::Decline,
        };

        {
            // 超时或已由 respond 处理：无论如何都清掉等待通道，避免残留。
            handle.inner.lock().unwrap().pending.remove(&request_id);
        }
        clear_incoming(&handle, &session_id, true);
        match decision {
            LanDecision::Decline => {
                let _ = decision_tx.send(PrepareUploadDecisionV2::Decline);
            }
            LanDecision::Accept | LanDecision::AcceptAndTrust => {
                let accepted = file_ids.clone();
                if decision_tx
                    .send(PrepareUploadDecisionV2::Accept(accepted))
                    .is_ok()
                {
                    if let Some(shared) = shared.upgrade() {
                        let settings = shared.settings.lock().unwrap().clone();
                        for (_, text) in &text_messages {
                            record_received_text(&app, &shared, &settings, text);
                            record_text_transfer(&handle, &info.alias, text);
                        }
                    }
                    if !file_ids.is_empty() {
                        let received_files = files
                            .iter()
                            .filter(|(id, _)| file_ids.contains(*id))
                            .map(|(id, file)| (id.clone(), file.clone()))
                            .collect::<HashMap<_, _>>();
                        if !received_files.is_empty() {
                            start_receive_session(
                                &handle,
                                &session_id,
                                &info.alias,
                                &received_files,
                            );
                        }
                    }
                }
            }
        }
        emit_state_with(&app, &handle, &shared);
    });
}

// 文本消息不传输文件，但仍记一条传输记录，便于界面反馈。
fn record_text_transfer(handle: &Arc<LanTransferHandle>, peer_alias: &str, text: &str) {
    handle.inner.lock().unwrap().push_transfer(TransferEntry {
        id: uuid::Uuid::new_v4().to_string(),
        direction: "receive",
        peer_alias: peer_alias.to_string(),
        status: "done",
        label: "text".into(),
        total_bytes: text.len() as u64,
        done_bytes: Arc::new(AtomicU64::new(text.len() as u64)),
        error: None,
        session_id: None,
        peer_host: None,
        cancel: None,
    });
}

// 记录一次接收会话，用于 upload 事件与进度展示。
fn start_receive_session(
    handle: &Arc<LanTransferHandle>,
    session_id: &str,
    peer_alias: &str,
    files: &HashMap<String, FileDto>,
) {
    let transfer_id = uuid::Uuid::new_v4().to_string();
    let total_bytes = files.values().map(|file| file.size).sum();
    let label = if files.len() == 1 {
        files
            .values()
            .next()
            .map(|file| file.file_name.clone())
            .unwrap_or_else(|| "file".into())
    } else {
        format!("{} files", files.len())
    };

    let mut inner = handle.inner.lock().unwrap();
    inner.push_transfer(TransferEntry {
        id: transfer_id.clone(),
        direction: "receive",
        peer_alias: peer_alias.to_string(),
        status: "active",
        label,
        total_bytes,
        done_bytes: Arc::new(AtomicU64::new(0)),
        error: None,
        session_id: Some(session_id.to_string()),
        peer_host: None,
        cancel: None,
    });
    inner.sessions.insert(
        session_id.to_string(),
        ReceiveSession {
            transfer_id,
            files: files.clone(),
        },
    );
}

fn finish_session(
    app: &AppHandle,
    handle: &Arc<LanTransferHandle>,
    shared: &Weak<SharedState>,
    session_id: &str,
    reason: SessionEndReasonV2,
) {
    let session = handle.inner.lock().unwrap().sessions.remove(session_id);
    if let Some(session) = session {
        let status = match reason {
            SessionEndReasonV2::Finished => "done",
            SessionEndReasonV2::Cancelled => "cancelled",
        };
        handle
            .inner
            .lock()
            .unwrap()
            .finish_transfer(&session.transfer_id, status, None);
    }
    emit_state_with(app, handle, shared);
}

fn clear_incoming(handle: &Arc<LanTransferHandle>, session_id: &str, drop_pending: bool) {
    let mut inner = handle.inner.lock().unwrap();
    let matches_session = inner
        .incoming
        .as_ref()
        .is_some_and(|entry| entry.session_id == session_id);
    if !matches_session {
        return;
    }
    if let Some(entry) = inner.incoming.take() {
        if drop_pending {
            inner.pending.remove(&entry.request_id);
        }
    }
}

// 处理单个文件的上传：文本包与图片进内存还原，其余直接写入保存目录。
fn handle_file_upload(
    app: &AppHandle,
    handle: &Arc<LanTransferHandle>,
    shared: &Weak<SharedState>,
    session_id: String,
    file_id: String,
    file: FileDto,
    target_tx: oneshot::Sender<FileUploadTarget>,
) {
    let (transfer_id, file_name) = {
        let inner = handle.inner.lock().unwrap();
        let Some(session) = inner.sessions.get(&session_id) else {
            // 未知会话：丢弃应答会让请求以 500 结束。
            return;
        };
        let file_name = session
            .files
            .get(&file_id)
            .map(|entry| entry.file_name.clone())
            .unwrap_or_else(|| file.file_name.clone());
        (session.transfer_id.clone(), file_name)
    };

    let Some(shared) = shared.upgrade() else {
        return;
    };
    let settings = shared.settings.lock().unwrap().clone();
    let is_text_file = is_text_package(&file_name);
    let is_image = file.file_type.starts_with("image/");
    let in_memory = (is_text_file && file.size <= TEXT_RESTORE_MAX_BYTES as u64)
        || (is_image && file.size as usize <= settings.max_image_bytes);

    if in_memory {
        let (binary_tx, mut binary_rx) = mpsc::channel::<Bytes>(16);
        let (result_tx, result_rx) = oneshot::channel::<Result<(), String>>();
        let app = app.clone();
        let handle = handle.clone();
        let shared_for_task = shared.clone();
        let transfer_id_for_task = transfer_id.clone();
        tokio::spawn(async move {
            let mut buffer: Vec<u8> = Vec::new();
            while let Some(chunk) = binary_rx.recv().await {
                buffer.extend_from_slice(&chunk);
            }

            if buffer.len() as u64 != file.size {
                let _ = result_tx.send(Err(format!(
                    "Expected {} bytes, received {}",
                    file.size,
                    buffer.len()
                )));
                handle.inner.lock().unwrap().finish_transfer(
                    &transfer_id_for_task,
                    "failed",
                    Some("size_mismatch".into()),
                );
                emit_state_with(&app, &handle, &Arc::downgrade(&shared_for_task));
                return;
            }

            let outcome = if is_text_file {
                let text = String::from_utf8_lossy(&buffer).to_string();
                record_received_text(
                    &app,
                    &shared_for_task,
                    &shared_for_task.settings.lock().unwrap().clone(),
                    &text,
                );
                Ok(())
            } else {
                record_received_image(
                    &app,
                    &shared_for_task,
                    &shared_for_task.settings.lock().unwrap().clone(),
                    &file.file_type,
                    &buffer,
                )
            };

            match outcome {
                Ok(()) => {
                    let _ = result_tx.send(Ok(()));
                    handle.inner.lock().unwrap().finish_transfer(
                        &transfer_id_for_task,
                        "done",
                        None,
                    );
                }
                Err(error) => {
                    let message = error.to_string();
                    let _ = result_tx.send(Err(message.clone()));
                    handle.inner.lock().unwrap().finish_transfer(
                        &transfer_id_for_task,
                        "failed",
                        Some(message),
                    );
                }
            }
            emit_state_with(&app, &handle, &Arc::downgrade(&shared_for_task));
        });

        let _ = target_tx.send(FileUploadTarget::Stream {
            binary_tx,
            result_rx,
        });
        return;
    }

    let dir = match resolve_download_dir(app, &shared) {
        Ok(dir) => dir,
        Err(error) => {
            let mut inner = handle.inner.lock().unwrap();
            inner.warning = Some(error.to_string());
            drop(inner);
            return;
        }
    };
    let path = util::unique_file_path(&dir, &util::sanitize_file_name(&file_name));
    let (progress_tx, mut progress_rx) = mpsc::channel::<u64>(16);
    let (result_tx, result_rx) = oneshot::channel::<Result<(), String>>();

    {
        let done_bytes = {
            let inner = handle.inner.lock().unwrap();
            inner
                .transfers
                .iter()
                .find(|entry| entry.id == transfer_id)
                .map(|entry| entry.done_bytes.clone())
        };
        if let Some(done_bytes) = done_bytes {
            let app = app.clone();
            let handle = handle.clone();
            let shared = Arc::downgrade(&shared);
            tokio::spawn(async move {
                let mut last_emit = Instant::now();
                while let Some(written) = progress_rx.recv().await {
                    done_bytes.store(written, Ordering::Relaxed);
                    if last_emit.elapsed() >= Duration::from_millis(200) {
                        last_emit = Instant::now();
                        emit_state_with(&app, &handle, &shared);
                    }
                }
            });
        }
    }

    let app = app.clone();
    let handle = handle.clone();
    let shared_for_task = shared.clone();
    let path_for_task = path.clone();
    let size = file.size;
    let stored_name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or(file_name);
    let peer_alias = session_peer_alias(&handle, &session_id);
    tokio::spawn(async move {
        let result = result_rx
            .await
            .unwrap_or_else(|_| Err("upload aborted".into()));
        match result {
            Ok(()) => {
                let mut inner = handle.inner.lock().unwrap();
                inner.finish_transfer(&transfer_id, "done", None);
                inner.push_received(super::ReceivedEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    file_name: stored_name,
                    path: path_for_task,
                    size,
                    from_alias: peer_alias,
                    received_at_ms: util::now_ms(),
                });
            }
            Err(error) => {
                handle
                    .inner
                    .lock()
                    .unwrap()
                    .finish_transfer(&transfer_id, "failed", Some(error));
            }
        }
        emit_state_with(&app, &handle, &Arc::downgrade(&shared_for_task));
    });

    let _ = target_tx.send(FileUploadTarget::Path {
        path,
        result_tx,
        progress_tx: Some(progress_tx),
    });
}

// 通过会话 ID 取回发送方别名，用于接收记录展示。
fn session_peer_alias(handle: &Arc<LanTransferHandle>, session_id: &str) -> String {
    let inner = handle.inner.lock().unwrap();
    inner
        .sessions
        .get(session_id)
        .and_then(|session| {
            inner
                .transfers
                .iter()
                .find(|entry| entry.id == session.transfer_id)
        })
        .map(|entry| entry.peer_alias.clone())
        .unwrap_or_default()
}

// 解析接收目录：优先使用设置项，否则回退系统下载目录。
fn resolve_download_dir(app: &AppHandle, shared: &Arc<SharedState>) -> Result<PathBuf> {
    let configured = shared
        .settings
        .lock()
        .unwrap()
        .lan_transfer_download_dir
        .clone();
    let path = match configured {
        Some(path) => PathBuf::from(path),
        None => {
            use tauri::Manager;
            app.path().download_dir()?
        }
    };
    util::validate_download_dir(&path)?;
    Ok(path)
}

// 把收到的文本写入剪贴板与历史。
fn record_received_text(
    app: &AppHandle,
    shared: &Arc<SharedState>,
    settings: &crate::models::AppSettings,
    text: &str,
) {
    if text.trim().is_empty() {
        return;
    }

    if let Err(error) = write_text_to_history(app, shared, settings, text) {
        shared.lan_transfer.inner.lock().unwrap().warning = Some(error.to_string());
    }
}

fn write_text_to_history(
    app: &AppHandle,
    shared: &Arc<SharedState>,
    settings: &crate::models::AppSettings,
    text: &str,
) -> Result<()> {
    let capture = build_captured_clipboard(
        settings,
        text.to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )?
    .context("empty text payload")?;
    let item = {
        let mut store = shared.history_store.lock().unwrap();
        store_capture_item(
            &mut store,
            capture,
            Some(("LocalSend".into(), None)),
            settings,
        )?
    };
    let _ = app.emit(HISTORY_UPDATED_EVENT, history_item_to_dto(&item));
    crate::sync::schedule_auto_sync(app.clone(), shared.clone());
    crate::capture::mark_clipboard_suppressed(shared, item.hash.clone());
    write_received_item_to_clipboard(app, &item)?;
    Ok(())
}

// 把收到的图片写入剪贴板与历史。
fn record_received_image(
    app: &AppHandle,
    shared: &Arc<SharedState>,
    settings: &crate::models::AppSettings,
    mime_type: &str,
    bytes: &[u8],
) -> Result<()> {
    let decoded = image::load_from_memory(bytes).context("unsupported image payload")?;
    let (width, height) = decoded.dimensions();
    let png_bytes = encode_png_for_storage(decoded, settings.max_image_bytes)?;

    let capture = build_captured_clipboard(
        settings,
        String::new(),
        None,
        None,
        Some(png_bytes),
        Some(bytes.to_vec()),
        Some(mime_type.to_string()),
        Some(width),
        Some(height),
    )?
    .context("empty image payload")?;
    let item = {
        let mut store = shared.history_store.lock().unwrap();
        store_capture_item(
            &mut store,
            capture,
            Some(("LocalSend".into(), None)),
            settings,
        )?
    };
    let _ = app.emit(HISTORY_UPDATED_EVENT, history_item_to_dto(&item));
    crate::sync::schedule_auto_sync(app.clone(), shared.clone());
    crate::capture::mark_clipboard_suppressed(shared, item.hash.clone());
    write_received_item_to_clipboard(app, &item)?;
    Ok(())
}

// 统一的剪贴板写入：Windows 图片走原生写入，其余走通用链路。
fn write_received_item_to_clipboard(
    app: &AppHandle,
    item: &crate::models::StoredClipboardItem,
) -> Result<()> {
    #[cfg(windows)]
    {
        if item.kind == "image" {
            let png_bytes = item.image_png.as_deref().context("image payload missing")?;
            crate::clipboard::write_image_to_clipboard(png_bytes)?;
            return Ok(());
        }
    }

    write_item_to_clipboard_with_profile(app, item, TargetProfile::Generic)
        .map(|_| ())
        .map_err(|error| anyhow::anyhow!(error.to_string()))
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
