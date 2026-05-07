use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::models::{
    LanReceiverSession, LanReceiverStateDto, LanTransferMessage, LanTransferMessageDto,
};

const ACTIVE_DEVICE_WINDOW: Duration = Duration::from_secs(15);

pub(crate) fn system_time_ms(value: SystemTime) -> Option<u64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}

pub(crate) fn now_ms() -> u64 {
    system_time_ms(SystemTime::now()).unwrap_or(0)
}

pub(crate) fn message_to_dto(
    message: &LanTransferMessage,
    token: &str,
    escape_url_component: fn(&str) -> String,
) -> LanTransferMessageDto {
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

pub(crate) fn receiver_state_dto(
    session: Option<&LanReceiverSession>,
    escape_url_component: fn(&str) -> String,
) -> LanReceiverStateDto {
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
            .map(|message| message_to_dto(message, &session.token, escape_url_component))
            .collect(),
    }
}
