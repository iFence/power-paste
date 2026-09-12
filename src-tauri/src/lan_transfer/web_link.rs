//! 浏览器扫码页：分享给手机（下载页）与手机传给我（上传页）。
//!
//! 浏览器无法校验自签证书，因此链接模式一律以明文 HTTP 提供服务，关闭后恢复加密服务。

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Weak},
    time::Duration,
};

use anyhow::{Context, Result};
use localsend::{
    http::server::{
        web::{WebConfig, WebSendConfig, WebSendEvent},
        ServerHandle, TlsConfig,
    },
    model::transfer::FileContent,
};
use qrcode::{render::svg, QrCode};
use tauri::AppHandle;
use tokio::sync::mpsc;

use super::{
    emit_state_with,
    identity::LanIdentity,
    send::{SendFile, SendSource},
    LanTransferHandle,
};
use crate::models::SharedState;

// 自定义页面：保持 power-paste 的外观，并支持文本消息的收发。
const DOWNLOAD_HTML: &str = include_str!("../../assets/lan-web/download.html");
const UPLOAD_HTML: &str = include_str!("../../assets/lan-web/upload.html");

// 组装指定模式的服务端配置；返回 TLS 配置与被分享文件的路径表。
#[allow(clippy::type_complexity)]
pub(super) fn build_config(
    mode: &str,
    files: Vec<SendFile>,
    web_tx: mpsc::Sender<WebSendEvent>,
    identity: &LanIdentity,
    pin: Option<String>,
) -> (Option<TlsConfig>, WebConfig, HashMap<String, PathBuf>) {
    match mode {
        "share" => {
            let mut dtos = HashMap::new();
            let mut paths = HashMap::new();
            for picked in files {
                // 分享页需要真实路径；内存内容（文本消息）不参与网页下载。
                let SendFile { id, file, source } = picked;
                let SendSource::Path(path) = source else {
                    continue;
                };
                dtos.insert(id.clone(), file);
                paths.insert(id, path);
            }
            (
                None,
                WebConfig {
                    send: Some(WebSendConfig {
                        files: dtos,
                        pin,
                        event_tx: web_tx,
                    }),
                    upload: false,
                    i18n: Default::default(),
                    pages: localsend::http::server::web::WebPages {
                        download_html: Some(DOWNLOAD_HTML.to_string()),
                        upload_html: None,
                        error_403_html: None,
                    },
                },
                paths,
            )
        }
        "receive" => (
            None,
            WebConfig {
                send: None,
                upload: true,
                i18n: Default::default(),
                pages: localsend::http::server::web::WebPages {
                    download_html: None,
                    upload_html: Some(UPLOAD_HTML.to_string()),
                    error_403_html: None,
                },
            },
            HashMap::new(),
        ),
        _ => (
            Some(identity.tls_config()),
            WebConfig {
                send: None,
                upload: false,
                i18n: Default::default(),
                pages: Default::default(),
            },
            HashMap::new(),
        ),
    }
}

// 计算浏览器访问地址与二维码。
pub(super) fn page_link(server: &ServerHandle) -> Result<(String, String)> {
    // 服务端给出的地址按字典序排序，首项可能是 docker0 / VPN 等虚拟网卡，这里按网卡类型挑一次。
    let address = super::util::preferred_lan_address(&server.local_addresses())
        .context("lan_transfer_no_interface")?;
    let url = format!("http://{address}");
    let qr_svg = build_qr_svg(&url)?;
    Ok((url, qr_svg))
}

fn build_qr_svg(url: &str) -> Result<String> {
    let code = QrCode::new(url.as_bytes())?;
    Ok(code
        .render::<svg::Color<'_>>()
        .min_dimensions(200, 200)
        .dark_color(svg::Color("#1d232d"))
        .light_color(svg::Color("#ffffff"))
        .build())
}

// 浏览器发起的下载请求：全部接受，并按下发的文件表提供内容。
pub(crate) fn spawn_web_events(
    app: AppHandle,
    handle: Arc<LanTransferHandle>,
    shared: Weak<SharedState>,
    mut rx: mpsc::Receiver<WebSendEvent>,
) {
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                WebSendEvent::PrepareDownload { decision_tx, .. } => {
                    let _ = decision_tx.send(true);
                }
                WebSendEvent::FileDownload {
                    file_id,
                    content_tx,
                    ..
                } => {
                    let path = {
                        let inner = handle.inner.lock().unwrap();
                        inner.web_paths.get(&file_id).cloned()
                    };
                    let Some(path) = path else {
                        // 分享已结束：丢弃应答会让下载失败。
                        continue;
                    };
                    let _ = content_tx.send(FileContent::Path(path));
                    emit_state_with(&app, &handle, &shared);
                }
            }
        }
    });
}

// 停止旧服务并启动新模式的服务，随后刷新地址与二维码。
pub(super) async fn restart_server(
    handle: &Arc<LanTransferHandle>,
    identity: &Arc<LanIdentity>,
    mode: &str,
    files: Vec<SendFile>,
    pin: Option<String>,
) -> Result<()> {
    let (old_server, old_stop, server_tx, web_tx) = {
        let mut inner = handle.inner.lock().unwrap();
        let service = inner.service.as_mut().context("lan_transfer_not_running")?;
        (
            service.server.clone(),
            service.server_stop.take(),
            service.server_tx.clone(),
            service.web_tx.clone(),
        )
    };

    if let Some(stop) = old_stop {
        let _ = stop.send(());
    }
    let _ = tokio::time::timeout(Duration::from_secs(3), old_server.wait_stopped()).await;

    let (tls_config, web_config, paths) = build_config(mode, files, web_tx, identity, pin.clone());
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let server = localsend::http::server::start_with_port(
        identity.port,
        tls_config,
        identity.client_info(),
        None,
        Some(localsend::http::server::ServerConfigV2 {
            pin,
            verify_checksums: true,
            event_tx: server_tx,
        }),
        Some(web_config),
        stop_rx,
    )
    .await?;
    let server = Arc::new(server);

    let link = if mode == "none" {
        None
    } else {
        Some(page_link(&server)?)
    };

    let mut inner = handle.inner.lock().unwrap();
    if let Some(service) = inner.service.as_mut() {
        service.server = server;
        service.server_stop = Some(stop_tx);
    }
    inner.web_paths = paths;
    inner.web.mode = mode.to_string();
    inner.web.url = link.as_ref().map(|(url, _)| url.clone());
    inner.web.qr_svg = link.map(|(_, qr)| qr);
    Ok(())
}
