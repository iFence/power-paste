//! 发送链路：把本地文件或文本发送到已发现的 LocalSend 设备。

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{atomic::Ordering, Arc, Weak},
    time::Duration,
};

use bytes::Bytes;
use futures_util::StreamExt;
use localsend::{
    http::{
        client::{v2::LsHttpClientV2, ClientError},
        dto_v2::PrepareUploadRequestDtoV2,
    },
    model::{
        discovery::ProtocolType,
        transfer::{FileContent, FileDto, FileMetadata},
    },
    reqwest,
};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;

use super::{emit_state_with, identity::LanIdentity, util, LanTransferHandle};
use crate::models::{AppError, SharedState};

// 待发送内容的来源：普通文件按路径流式读取，文本消息直接放在内存里。
pub(super) enum SendSource {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

// 一个待发送的条目：协议元数据与内容来源。
pub(super) struct SendFile {
    pub(super) id: String,
    pub(super) file: FileDto,
    pub(super) source: SendSource,
}

// 协议原生文本消息的文件名与类型；官方客户端据 fileType 识别为消息。
pub(super) const TEXT_MESSAGE_FILE_NAME: &str = "message.txt";
pub(super) const TEXT_MESSAGE_FILE_TYPE: &str = "text/plain";

// 组装一条协议原生文本消息：正文放在 preview 中，对端无需下载即可显示。
pub(super) fn text_message(text: &str) -> SendFile {
    let id = uuid::Uuid::new_v4().to_string();
    SendFile {
        id: id.clone(),
        file: FileDto {
            id,
            file_name: TEXT_MESSAGE_FILE_NAME.into(),
            size: text.len() as u64,
            file_type: TEXT_MESSAGE_FILE_TYPE.into(),
            sha256: None,
            preview: Some(text.to_string()),
            metadata: None,
        },
        source: SendSource::Bytes(text.as_bytes().to_vec()),
    }
}

// 目标设备的连接信息。
pub(super) struct SendTarget {
    pub(super) alias: String,
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) protocol: ProtocolType,
    pub(super) fingerprint: String,
    // 对端要求的 PIN；None 表示未配置，若对端仍要求 PIN 会返回 pin_required。
    pub(super) pin: Option<String>,
}

// 收集待发送文件；不可读的路径会被跳过。
pub(super) fn collect_files(paths: &[String]) -> Result<Vec<SendFile>, AppError> {
    let mut picked = Vec::new();
    for path in paths {
        let path = PathBuf::from(path);
        let metadata = match std::fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => metadata,
            _ => continue,
        };
        let file_name = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "transfer-file".into());
        let id = uuid::Uuid::new_v4().to_string();
        let file_type = util::infer_mime_type(&path);
        picked.push(SendFile {
            id: id.clone(),
            file: FileDto {
                id,
                file_name,
                size: metadata.len(),
                file_type,
                sha256: None,
                preview: None,
                metadata: FileMetadata::from_fs_metadata(&metadata),
            },
            source: SendSource::Path(path),
        });
    }
    Ok(picked)
}

// 启动一次发送（文件或文本消息），进度通过状态事件回传前端。
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_send(
    app: AppHandle,
    handle: Arc<LanTransferHandle>,
    shared: Arc<SharedState>,
    identity: Arc<LanIdentity>,
    target: SendTarget,
    advertised_protocol: ProtocolType,
    transfer_id: String,
    files: Vec<SendFile>,
) {
    spawn_progress_ticker(app.clone(), handle.clone(), Arc::downgrade(&shared), transfer_id.clone());

    tauri::async_runtime::spawn(async move {
        let result = run_send(
            &handle,
            &identity,
            &target,
            advertised_protocol,
            &transfer_id,
            files,
        )
        .await;
        let mut inner = handle.inner.lock().unwrap();
        match result {
            Ok(()) => inner.finish_transfer(&transfer_id, "done", None),
            Err(SendFailure::Cancelled) => inner.finish_transfer(&transfer_id, "cancelled", None),
            Err(SendFailure::Failed(message)) => {
                inner.finish_transfer(&transfer_id, "failed", Some(message))
            }
        }
        drop(inner);
        emit_state_with(&app, &handle, &Arc::downgrade(&shared));
    });
}

// 取消一次发送。
pub(super) fn cancel_transfer(handle: &Arc<LanTransferHandle>, transfer_id: &str) {
    let token = {
        let inner = handle.inner.lock().unwrap();
        inner
            .transfers
            .iter()
            .find(|entry| entry.id == transfer_id)
            .and_then(|entry| entry.cancel.clone())
    };
    if let Some(token) = token {
        token.cancel();
    }
}

// 对端取消了接收：终止对应的发送任务。
pub(super) fn handle_peer_cancel(handle: &Arc<LanTransferHandle>, host: &str, session_id: &str) {
    let token = {
        let inner = handle.inner.lock().unwrap();
        inner
            .transfers
            .iter()
            .find(|entry| {
                entry.session_id.as_deref() == Some(session_id) && entry.peer_host.as_deref() == Some(host)
            })
            .and_then(|entry| entry.cancel.clone())
    };
    if let Some(token) = token {
        token.cancel();
    }
}

// 发送失败的原因，用于区分取消与真实错误。
enum SendFailure {
    Cancelled,
    Failed(String),
}

async fn run_send(
    handle: &Arc<LanTransferHandle>,
    identity: &Arc<LanIdentity>,
    target: &SendTarget,
    advertised_protocol: ProtocolType,
    transfer_id: &str,
    files: Vec<SendFile>,
) -> Result<(), SendFailure> {
    let mut payload_files = HashMap::new();
    let mut sources = HashMap::new();
    for entry in files {
        payload_files.insert(entry.id.clone(), entry.file);
        sources.insert(entry.id, entry.source);
    }

    let prepared = prepare_upload(
        handle,
        identity,
        target,
        advertised_protocol,
        transfer_id,
        payload_files.clone(),
    )
    .await?;
    let Some(response) = prepared else {
        // 对端以 204 应答：没有文件需要传输（例如文本消息已被当消息接收）。
        return Ok(());
    };

    let client = build_client(identity, target)?;
    let mut ids: Vec<String> = response.files.keys().cloned().collect();
    ids.sort_by_key(|id| {
        payload_files
            .get(id)
            .map(|file| file.file_name.clone())
            .unwrap_or_default()
    });

    let mut sent_bytes = 0u64;
    for file_id in ids {
        let Some(token) = response.files.get(&file_id) else {
            continue;
        };
        let Some(file) = payload_files.get(&file_id) else {
            continue;
        };
        // 对端把文本消息当成普通文件接受时，这里会把正文真实上传，避免对端会话悬挂。
        let Some(source) = sources.remove(&file_id) else {
            continue;
        };

        let cancel = current_cancel(handle, transfer_id).unwrap_or_default();
        let body = build_upload_body(
            handle.clone(),
            transfer_id.to_string(),
            source,
            sent_bytes,
        );

        match client
            .upload(
                target.protocol,
                &target.host,
                target.port,
                None,
                &response.session_id,
                &file_id,
                token,
                body,
                cancel.clone(),
            )
            .await
        {
            Ok(()) => {
                sent_bytes += file.size;
                set_done_bytes(handle, transfer_id, sent_bytes);
            }
            Err(ClientError::Cancelled) => {
                let _ = client
                    .cancel(
                        target.protocol,
                        &target.host,
                        target.port,
                        &response.session_id,
                    )
                    .await;
                return Err(SendFailure::Cancelled);
            }
            Err(error) => {
                let _ = client
                    .cancel(
                        target.protocol,
                        &target.host,
                        target.port,
                        &response.session_id,
                    )
                    .await;
                return Err(SendFailure::Failed(format!("{}: {error}", file.file_name)));
            }
        }
    }

    Ok(())
}

// 发起 prepare-upload；返回 None 表示对端未接受任何文件。
async fn prepare_upload(
    handle: &Arc<LanTransferHandle>,
    identity: &Arc<LanIdentity>,
    target: &SendTarget,
    advertised_protocol: ProtocolType,
    transfer_id: &str,
    files: HashMap<String, FileDto>,
) -> Result<Option<localsend::http::dto_v2::PrepareUploadResponseDtoV2>, SendFailure> {
    let client = build_client(identity, target)?;
    let cancel = CancellationToken::new();
    {
        let mut inner = handle.inner.lock().unwrap();
        if let Some(entry) = inner.transfers.iter_mut().find(|entry| entry.id == transfer_id) {
            entry.cancel = Some(cancel.clone());
            entry.peer_host = Some(target.host.clone());
        }
    }

    let payload = PrepareUploadRequestDtoV2 {
        info: identity.register_dto(advertised_protocol),
        files,
    };

    match client
        .prepare_upload(
            target.protocol,
            &target.host,
            target.port,
            None,
            payload,
            target.pin.as_deref(),
            cancel,
        )
        .await
    {
        Ok(prepared) => {
            let Some(response) = prepared.response else {
                return Ok(None);
            };
            {
                let mut inner = handle.inner.lock().unwrap();
                if let Some(entry) =
                    inner.transfers.iter_mut().find(|entry| entry.id == transfer_id)
                {
                    entry.session_id = Some(response.session_id.clone());
                }
                // 记下可用的 PIN：同一设备后续发送不再要求用户重复输入。
                if let Some(pin) = target.pin.as_deref() {
                    inner
                        .device_pins
                        .insert(target.fingerprint.clone(), pin.to_string());
                }
            }
            Ok(Some(response))
        }
        Err(ClientError::Cancelled) => Err(SendFailure::Cancelled),
        Err(ClientError::StatusCode(error)) => {
            let reason = match error.status {
                401 => "pin_required".to_string(),
                403 => "declined".to_string(),
                409 => "busy".to_string(),
                429 => "too_many_requests".to_string(),
                status => format!("status_{status}"),
            };
            Err(SendFailure::Failed(reason))
        }
        Err(error) => Err(SendFailure::Failed(error.to_string())),
    }
}

fn build_client(
    identity: &Arc<LanIdentity>,
    target: &SendTarget,
) -> Result<LsHttpClientV2, SendFailure> {
    let expected_fingerprint = match target.protocol {
        ProtocolType::Https => Some(target.fingerprint.clone()),
        ProtocolType::Http => None,
    };
    LsHttpClientV2::try_new(
        &identity.key_pem,
        &identity.cert_pem,
        expected_fingerprint,
        super::client_timeout_for(super::ClientPurpose::Transfer),
    )
    .map_err(|error| SendFailure::Failed(error.to_string()))
}

// 把待发送内容转成协议层的内容来源；内存内容通过单块流发送。
fn content_of(source: SendSource) -> FileContent {
    match source {
        SendSource::Path(path) => FileContent::Path(path),
        SendSource::Bytes(bytes) => {
            let (tx, rx) = mpsc::channel::<Bytes>(1);
            tokio::spawn(async move {
                let _ = tx.send(Bytes::from(bytes)).await;
            });
            FileContent::Stream(rx)
        }
    }
}

// 构造带进度的请求体，把已发送字节数写入传输记录。
fn build_upload_body(
    handle: Arc<LanTransferHandle>,
    transfer_id: String,
    source: SendSource,
    base_bytes: u64,
) -> reqwest::Body {
    let mut sent = 0u64;
    let stream = ReceiverStream::new(content_of(source).into_receiver()).map(move |chunk: Bytes| {
        sent += chunk.len() as u64;
        set_done_bytes(&handle, &transfer_id, base_bytes + sent);
        Ok::<Bytes, anyhow::Error>(chunk)
    });
    reqwest::Body::wrap_stream(stream)
}

fn set_done_bytes(handle: &Arc<LanTransferHandle>, transfer_id: &str, value: u64) {
    let inner = handle.inner.lock().unwrap();
    if let Some(entry) = inner.transfers.iter().find(|entry| entry.id == transfer_id) {
        entry.done_bytes.store(value, Ordering::Relaxed);
    }
}

fn current_cancel(
    handle: &Arc<LanTransferHandle>,
    transfer_id: &str,
) -> Option<CancellationToken> {
    let inner = handle.inner.lock().unwrap();
    inner
        .transfers
        .iter()
        .find(|entry| entry.id == transfer_id)
        .and_then(|entry| entry.cancel.clone())
}

// 传输进行期间按固定间隔推送进度，避免每个数据块都触发一次事件。
fn spawn_progress_ticker(
    app: AppHandle,
    handle: Arc<LanTransferHandle>,
    shared: Weak<SharedState>,
    transfer_id: String,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            let active = {
                let inner = handle.inner.lock().unwrap();
                inner
                    .transfers
                    .iter()
                    .find(|entry| entry.id == transfer_id)
                    .is_some_and(|entry| entry.status == "active")
            };
            if !active {
                break;
            }
            emit_state_with(&app, &handle, &shared);
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{text_message, SendSource, TEXT_MESSAGE_FILE_NAME, TEXT_MESSAGE_FILE_TYPE};

    #[test]
    fn builds_protocol_native_text_messages() {
        let text = "文本消息";
        let message = text_message(text);

        assert_eq!(message.file.file_name, TEXT_MESSAGE_FILE_NAME);
        // 官方客户端按 MIME 判断消息类型，必须是 text/plain 而不是简写 text。
        assert_eq!(message.file.file_type, TEXT_MESSAGE_FILE_TYPE);
        assert_eq!(message.file.preview.as_deref(), Some(text));
        assert_eq!(message.file.size, text.len() as u64);
        assert!(message.file.sha256.is_none());
        match message.source {
            SendSource::Bytes(bytes) => assert_eq!(bytes, text.as_bytes()),
            SendSource::Path(_) => panic!("text messages must not use a file path"),
        }
    }
}
