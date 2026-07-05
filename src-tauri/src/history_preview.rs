use std::sync::Arc;

use tauri::{
    http::{header, Response, StatusCode},
    Manager, Runtime, UriSchemeContext,
};

use crate::models::{SharedState, PANEL_LABEL};

pub(crate) const HISTORY_PREVIEW_PROTOCOL: &str = "history-preview";

fn empty_response(status: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .body(Vec::new())
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

fn history_id_from_path(path: &str) -> Option<&str> {
    let id = path.trim_start_matches('/').split('/').next()?.trim();
    if id.is_empty()
        || !id
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || value == '-' || value == '_')
    {
        return None;
    }

    Some(id)
}

pub(crate) fn handle_history_preview_request<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    if ctx.webview_label() != PANEL_LABEL {
        return empty_response(StatusCode::FORBIDDEN);
    }

    let Some(id) = history_id_from_path(request.uri().path()) else {
        return empty_response(StatusCode::BAD_REQUEST);
    };
    let Some(state) = ctx.app_handle().try_state::<Arc<SharedState>>() else {
        return empty_response(StatusCode::SERVICE_UNAVAILABLE);
    };

    let preview = state
        .history_store
        .lock()
        .unwrap()
        .preview_image_bytes(id)
        .ok()
        .flatten();

    let Some((mime, bytes)) = preview else {
        return empty_response(StatusCode::NOT_FOUND);
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "no-store")
        .body(bytes)
        .unwrap_or_else(|_| empty_response(StatusCode::INTERNAL_SERVER_ERROR))
}
