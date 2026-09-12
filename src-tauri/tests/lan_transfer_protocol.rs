//! 局域网互传的端到端协议测试：用与运行时相同的调用序列完成一次文件传输。

use std::{collections::HashMap, path::PathBuf, time::Duration};

use localsend::{
    crypto::cert::generate_self_signed,
    http::{
        client::{v2::LsHttpClientV2, ClientError},
        dto_v2::{PrepareUploadRequestDtoV2, RegisterDtoV2},
        server::{
            common::save::FileUploadTarget,
            start_with_port,
            v2::{PrepareUploadDecisionV2, ServerEventV2},
            ServerConfigV2, TlsConfig,
        },
        state::ClientInfo,
    },
    model::{
        discovery::{DeviceType, ProtocolType, PROTOCOL_VERSION_V2},
        transfer::FileDto,
    },
};
use tokio::sync::{mpsc, oneshot};

fn unique_temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "power-paste-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_millis())
            .unwrap_or_default()
    ))
}

#[tokio::test]
async fn receives_a_file_over_the_v2_protocol() {
    let receiver_cert = generate_self_signed().expect("receiver cert");
    let sender_cert = generate_self_signed().expect("sender cert");

    let (event_tx, mut event_rx) = mpsc::channel::<ServerEventV2>(16);
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let server = start_with_port(
        0,
        Some(TlsConfig {
            cert: receiver_cert.certificate_pem.clone(),
            private_key: receiver_cert.private_key_pem.clone(),
        }),
        ClientInfo {
            alias: "Receiver".into(),
            version: PROTOCOL_VERSION_V2.into(),
            device_model: Some("Power Paste".into()),
            device_type: Some(DeviceType::Desktop),
            token: receiver_cert.fingerprint.clone(),
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
    let port = server.port();

    let source_path = unique_temp_path("source");
    let target_path = unique_temp_path("target");
    let payload = b"power-paste lan transfer".to_vec();
    std::fs::write(&source_path, &payload).expect("write source file");

    // 接收端：接受请求、把内容写入目标文件，最后等待会话结束。
    let receiver_target = target_path.clone();
    let receiver_task = tokio::spawn(async move {
        let (result_tx, result_rx) = oneshot::channel::<Result<(), String>>();
        let mut result_tx = Some(result_tx);
        let mut session_id: Option<String> = None;
        while let Some(event) = event_rx.recv().await {
            match event {
                ServerEventV2::PrepareUpload {
                    session_id: id,
                    files,
                    decision_tx,
                    ..
                } => {
                    session_id = Some(id);
                    let accepted = files.keys().cloned().collect();
                    let _ = decision_tx.send(PrepareUploadDecisionV2::Accept(accepted));
                }
                ServerEventV2::FileUpload { target_tx, .. } => {
                    let Some(result_tx) = result_tx.take() else {
                        continue;
                    };
                    let _ = target_tx.send(FileUploadTarget::Path {
                        path: receiver_target.clone(),
                        result_tx,
                        progress_tx: None,
                    });
                }
                ServerEventV2::SessionEnd { .. } => break,
                _ => {}
            }
        }
        let saved = result_rx.await.expect("save result");
        (session_id, saved)
    });

    let client = LsHttpClientV2::try_new(
        &sender_cert.private_key_pem,
        &sender_cert.certificate_pem,
        Some(receiver_cert.fingerprint.clone()),
        Some(Duration::from_secs(10)),
    )
    .expect("client");

    let register = client
        .register(
            ProtocolType::Https,
            "127.0.0.1",
            port,
            RegisterDtoV2 {
                alias: "Sender".into(),
                version: PROTOCOL_VERSION_V2.into(),
                device_model: Some("Power Paste".into()),
                device_type: Some(DeviceType::Desktop),
                fingerprint: sender_cert.fingerprint.clone(),
                port: 0,
                protocol: ProtocolType::Https,
                download: false,
            },
        )
        .await
        .expect("register");
    assert_eq!(register.body.alias, "Receiver");

    let file_id = "file-1".to_string();
    let mut files = HashMap::new();
    files.insert(
        file_id.clone(),
        FileDto {
            id: file_id.clone(),
            file_name: "note.txt".into(),
            size: payload.len() as u64,
            file_type: "text/plain".into(),
            sha256: None,
            preview: None,
            metadata: None,
        },
    );

    let prepared = client
        .prepare_upload(
            ProtocolType::Https,
            "127.0.0.1",
            port,
            None,
            PrepareUploadRequestDtoV2 {
                info: RegisterDtoV2 {
                    alias: "Sender".into(),
                    version: PROTOCOL_VERSION_V2.into(),
                    device_model: Some("Power Paste".into()),
                    device_type: Some(DeviceType::Desktop),
                    fingerprint: sender_cert.fingerprint.clone(),
                    port: 0,
                    protocol: ProtocolType::Https,
                    download: false,
                },
                files,
            },
            None,
            tokio_util::sync::CancellationToken::new(),
        )
        .await
        .expect("prepare upload");
    let response = prepared.response.expect("accepted response");
    let token = response.files.get(&file_id).expect("file token").clone();

    client
        .upload(
            ProtocolType::Https,
            "127.0.0.1",
            port,
            None,
            &response.session_id,
            &file_id,
            &token,
            localsend::reqwest::Body::from(payload.clone()),
            tokio_util::sync::CancellationToken::new(),
        )
        .await
        .map_err(|error| match error {
            ClientError::Cancelled => panic!("upload cancelled"),
            other => panic!("upload failed: {other}"),
        })
        .expect("upload result");

    let (session_id, saved) = receiver_task.await.expect("receiver task");
    assert!(session_id.is_some(), "prepare-upload should start a session");
    assert!(saved.is_ok(), "receiver failed to save: {saved:?}");
    assert_eq!(std::fs::read(&target_path).expect("read target"), payload);

    let _ = stop_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(3), server.wait_stopped()).await;
    let _ = std::fs::remove_file(&source_path);
    let _ = std::fs::remove_file(&target_path);
}

// 扫码页模式会把服务切成明文 HTTP：此时 v2 端点必须仍然可用，
// 否则手机页面打开期间官方客户端无法与本机互传。
#[tokio::test]
async fn receives_a_file_over_plain_http() {
    let receiver_cert = generate_self_signed().expect("receiver cert");
    let sender_cert = generate_self_signed().expect("sender cert");

    let (event_tx, mut event_rx) = mpsc::channel::<ServerEventV2>(16);
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let server = start_with_port(
        0,
        None,
        ClientInfo {
            alias: "Receiver".into(),
            version: PROTOCOL_VERSION_V2.into(),
            device_model: Some("Power Paste".into()),
            device_type: Some(DeviceType::Desktop),
            token: receiver_cert.fingerprint.clone(),
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
    let port = server.port();

    let target_path = unique_temp_path("http-target");
    let payload = b"plain http lan transfer".to_vec();

    let receiver_target = target_path.clone();
    let receiver_task = tokio::spawn(async move {
        let (result_tx, result_rx) = oneshot::channel::<Result<(), String>>();
        let mut result_tx = Some(result_tx);
        while let Some(event) = event_rx.recv().await {
            match event {
                ServerEventV2::PrepareUpload {
                    files, decision_tx, ..
                } => {
                    let accepted = files.keys().cloned().collect();
                    let _ = decision_tx.send(PrepareUploadDecisionV2::Accept(accepted));
                }
                ServerEventV2::FileUpload { target_tx, .. } => {
                    let Some(result_tx) = result_tx.take() else {
                        continue;
                    };
                    let _ = target_tx.send(FileUploadTarget::Path {
                        path: receiver_target.clone(),
                        result_tx,
                        progress_tx: None,
                    });
                }
                ServerEventV2::SessionEnd { .. } => break,
                _ => {}
            }
        }
        result_rx.await.expect("save result")
    });

    let info = RegisterDtoV2 {
        alias: "Sender".into(),
        version: PROTOCOL_VERSION_V2.into(),
        device_model: Some("Power Paste".into()),
        device_type: Some(DeviceType::Desktop),
        fingerprint: sender_cert.fingerprint.clone(),
        port: 0,
        protocol: ProtocolType::Http,
        download: false,
    };
    // 明文连接不校验证书，因此也不需要固定指纹。
    let client = LsHttpClientV2::try_new(
        &sender_cert.private_key_pem,
        &sender_cert.certificate_pem,
        None,
        None,
    )
    .expect("client");

    let register = client
        .register(ProtocolType::Http, "127.0.0.1", port, info.clone())
        .await
        .expect("register");
    assert_eq!(register.body.alias, "Receiver");

    let file_id = "file-http".to_string();
    let mut files = HashMap::new();
    files.insert(
        file_id.clone(),
        FileDto {
            id: file_id.clone(),
            file_name: "note.txt".into(),
            size: payload.len() as u64,
            file_type: "text/plain".into(),
            sha256: None,
            preview: None,
            metadata: None,
        },
    );

    let prepared = client
        .prepare_upload(
            ProtocolType::Http,
            "127.0.0.1",
            port,
            None,
            PrepareUploadRequestDtoV2 { info, files },
            None,
            tokio_util::sync::CancellationToken::new(),
        )
        .await
        .expect("prepare upload");
    let response = prepared.response.expect("accepted response");
    let token = response.files.get(&file_id).expect("file token").clone();

    client
        .upload(
            ProtocolType::Http,
            "127.0.0.1",
            port,
            None,
            &response.session_id,
            &file_id,
            &token,
            localsend::reqwest::Body::from(payload.clone()),
            tokio_util::sync::CancellationToken::new(),
        )
        .await
        .expect("upload result");

    let saved = receiver_task.await.expect("receiver task");
    assert!(saved.is_ok(), "receiver failed to save: {saved:?}");
    assert_eq!(std::fs::read(&target_path).expect("read target"), payload);

    let _ = stop_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(3), server.wait_stopped()).await;
    let _ = std::fs::remove_file(&target_path);
}

#[tokio::test]
async fn rejects_an_unknown_file_upload() {
    let receiver_cert = generate_self_signed().expect("receiver cert");
    let (event_tx, mut event_rx) = mpsc::channel::<ServerEventV2>(16);
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let server = start_with_port(
        0,
        Some(TlsConfig {
            cert: receiver_cert.certificate_pem.clone(),
            private_key: receiver_cert.private_key_pem.clone(),
        }),
        ClientInfo {
            alias: "Receiver".into(),
            version: PROTOCOL_VERSION_V2.into(),
            device_model: Some("Power Paste".into()),
            device_type: Some(DeviceType::Desktop),
            token: receiver_cert.fingerprint.clone(),
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
    let port = server.port();

    // 接收端拒绝请求：发送端应当拿到 403。
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            if let ServerEventV2::PrepareUpload { decision_tx, .. } = event {
                let _ = decision_tx.send(PrepareUploadDecisionV2::Decline);
            }
        }
    });

    let sender_cert = generate_self_signed().expect("sender cert");
    let client = LsHttpClientV2::try_new(
        &sender_cert.private_key_pem,
        &sender_cert.certificate_pem,
        Some(receiver_cert.fingerprint.clone()),
        Some(Duration::from_secs(10)),
    )
    .expect("client");

    let mut files = HashMap::new();
    files.insert(
        "file-1".to_string(),
        FileDto {
            id: "file-1".into(),
            file_name: "note.txt".into(),
            size: 4,
            file_type: "text/plain".into(),
            sha256: None,
            preview: None,
            metadata: None,
        },
    );

    let result = client
        .prepare_upload(
            ProtocolType::Https,
            "127.0.0.1",
            port,
            None,
            PrepareUploadRequestDtoV2 {
                info: RegisterDtoV2 {
                    alias: "Sender".into(),
                    version: PROTOCOL_VERSION_V2.into(),
                    device_model: Some("Power Paste".into()),
                    device_type: Some(DeviceType::Desktop),
                    fingerprint: sender_cert.fingerprint.clone(),
                    port: 0,
                    protocol: ProtocolType::Https,
                    download: false,
                },
                files,
            },
            None,
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
    match result {
        Err(ClientError::StatusCode(status)) => assert_eq!(status.status, 403),
        Err(other) => panic!("unexpected error: {other}"),
        Ok(_) => panic!("declined request must fail"),
    }

    let _ = stop_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(3), server.wait_stopped()).await;
}

// 设置更新是同步 tauri command，运行在没有 tokio 上下文的线程上；
// 这里锁住"必须用 tauri 的异步运行时派发任务"这一约定，避免再次出现
// 直接调用 tokio::spawn 导致进程 panic 退出的问题。
#[test]
fn spawns_work_without_a_tokio_context() {
    let handle = tauri::async_runtime::spawn(async { 1u8 });
    let value = tauri::async_runtime::block_on(handle).expect("join spawned task");
    assert_eq!(value, 1);
}
