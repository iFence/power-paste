use std::sync::Arc;

use anyhow::Result;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;

use crate::models::{
    AppError, SharedState, UpdateDebugStatePayload, UpdateStatus, UPDATE_STATUS_EVENT,
};

fn emit_status(app: &AppHandle, shared: &Arc<SharedState>, next: UpdateStatus) -> UpdateStatus {
    *shared.update_status.lock().unwrap() = next.clone();
    let _ = app.emit(UPDATE_STATUS_EVENT, &next);
    next
}

fn current_status(shared: &Arc<SharedState>) -> UpdateStatus {
    shared.update_status.lock().unwrap().clone()
}

fn version_of(app: &AppHandle) -> String {
    app.package_info().version.to_string()
}

fn active_debug_status(shared: &Arc<SharedState>) -> Option<UpdateStatus> {
    if !cfg!(debug_assertions) {
        return None;
    }

    shared.update_debug_override.lock().unwrap().clone()
}

fn build_debug_status(
    current_version: String,
    payload: UpdateDebugStatePayload,
) -> Result<UpdateStatus, AppError> {
    let status = payload.status.trim().to_string();
    if !matches!(
        status.as_str(),
        "idle" | "checking" | "available" | "downloading" | "downloaded" | "up_to_date" | "error"
    ) {
        return Err(AppError::Message("invalid_update_debug_status".into()));
    }

    Ok(UpdateStatus {
        status,
        current_version,
        latest_version: payload.latest_version,
        body: payload.body,
        published_at: payload.published_at,
        downloaded_bytes: payload.downloaded_bytes,
        content_length: payload.content_length,
        error: payload.error,
    })
}

pub(crate) fn spawn_startup_check(app: AppHandle, shared: Arc<SharedState>) {
    tauri::async_runtime::spawn(async move {
        let _ = check_for_updates_inner(app, shared).await;
    });
}

pub(crate) fn spawn_manual_check(app: AppHandle, shared: Arc<SharedState>) {
    tauri::async_runtime::spawn(async move {
        let _ = check_for_updates_inner(app, shared).await;
    });
}

async fn check_for_updates_inner(app: AppHandle, shared: Arc<SharedState>) -> Result<UpdateStatus> {
    if let Some(next) = active_debug_status(&shared) {
        *shared.pending_update.lock().unwrap() = None;
        return Ok(emit_status(&app, &shared, next));
    }

    let current = current_status(&shared);
    if matches!(current.status.as_str(), "checking" | "downloading") {
        return Ok(current);
    }

    emit_status(
        &app,
        &shared,
        UpdateStatus {
            status: "checking".into(),
            current_version: version_of(&app),
            latest_version: None,
            body: None,
            published_at: None,
            downloaded_bytes: None,
            content_length: None,
            error: None,
        },
    );

    let result = app.updater()?.check().await;

    if let Some(next) = active_debug_status(&shared) {
        *shared.pending_update.lock().unwrap() = None;
        return Ok(emit_status(&app, &shared, next));
    }

    match result {
        Ok(Some(update)) => {
            let next = UpdateStatus {
                status: "available".into(),
                current_version: version_of(&app),
                latest_version: Some(update.version.clone()),
                body: update.body.clone(),
                published_at: update.date.as_ref().map(ToString::to_string),
                downloaded_bytes: None,
                content_length: None,
                error: None,
            };
            *shared.pending_update.lock().unwrap() = Some(update);
            Ok(emit_status(&app, &shared, next))
        }
        Ok(None) => {
            *shared.pending_update.lock().unwrap() = None;
            Ok(emit_status(
                &app,
                &shared,
                UpdateStatus {
                    status: "up_to_date".into(),
                    current_version: version_of(&app),
                    latest_version: None,
                    body: None,
                    published_at: None,
                    downloaded_bytes: None,
                    content_length: None,
                    error: None,
                },
            ))
        }
        Err(error) => {
            *shared.pending_update.lock().unwrap() = None;
            Ok(emit_status(
                &app,
                &shared,
                UpdateStatus {
                    status: "error".into(),
                    current_version: version_of(&app),
                    latest_version: None,
                    body: None,
                    published_at: None,
                    downloaded_bytes: None,
                    content_length: None,
                    error: Some(error.to_string()),
                },
            ))
        }
    }
}

#[tauri::command]
pub(crate) fn get_update_state(
    state: State<'_, Arc<SharedState>>,
) -> Result<UpdateStatus, AppError> {
    Ok(current_status(state.inner()))
}

#[tauri::command]
pub(crate) async fn check_for_updates(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<UpdateStatus, AppError> {
    check_for_updates_inner(app, state.inner().clone())
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn install_update(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<UpdateStatus, AppError> {
    let shared = state.inner().clone();
    if cfg!(debug_assertions) && active_debug_status(&shared).is_some() {
        let current = current_status(&shared);
        let downloaded_bytes = current
            .downloaded_bytes
            .or(current.content_length)
            .or(Some(100));
        let downloaded_status = UpdateStatus {
            status: "downloaded".into(),
            current_version: version_of(&app),
            latest_version: current.latest_version,
            body: current.body,
            published_at: current.published_at,
            downloaded_bytes,
            content_length: current.content_length.or(downloaded_bytes),
            error: None,
        };
        return Ok(emit_status(&app, &shared, downloaded_status));
    }

    let pending = shared
        .pending_update
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| AppError::Message("No update is ready to install".into()))?;

    let mut next = current_status(&shared);
    if next.latest_version.is_none() {
        next.latest_version = Some(pending.version.clone());
    }
    next.status = "downloading".into();
    next.error = None;
    next.downloaded_bytes = Some(0);
    next.content_length = None;
    emit_status(&app, &shared, next.clone());

    let app_for_progress = app.clone();
    let shared_for_progress = shared.clone();
    let latest_version = pending.version.clone();
    let published_at = pending.date.as_ref().map(ToString::to_string);
    let body = pending.body.clone();
    let mut downloaded = 0u64;

    if let Err(error) = pending
        .download_and_install(
            move |chunk_length, content_length| {
                downloaded += chunk_length as u64;
                let progress = UpdateStatus {
                    status: "downloading".into(),
                    current_version: version_of(&app_for_progress),
                    latest_version: Some(latest_version.clone()),
                    body: body.clone(),
                    published_at: published_at.clone(),
                    downloaded_bytes: Some(downloaded),
                    content_length: content_length.map(|value| value as u64),
                    error: None,
                };
                emit_status(&app_for_progress, &shared_for_progress, progress);
            },
            || {},
        )
        .await
    {
        return Ok(emit_status(
            &app,
            &shared,
            UpdateStatus {
                status: "error".into(),
                current_version: version_of(&app),
                latest_version: next.latest_version,
                body: next.body,
                published_at: next.published_at,
                downloaded_bytes: next.downloaded_bytes,
                content_length: next.content_length,
                error: Some(error.to_string()),
            },
        ));
    }

    let downloaded_status = UpdateStatus {
        status: "downloaded".into(),
        current_version: version_of(&app),
        latest_version: next.latest_version,
        body: next.body,
        published_at: next.published_at,
        downloaded_bytes: next.downloaded_bytes,
        content_length: next.content_length,
        error: None,
    };

    #[cfg(windows)]
    {
        return Ok(emit_status(&app, &shared, downloaded_status));
    }

    #[cfg(not(windows))]
    {
        emit_status(&app, &shared, downloaded_status);
        app.restart();
    }
}

// 仅在开发环境中设置或清除更新状态调试覆盖，payload 为 null 时恢复真实更新逻辑。
#[tauri::command]
pub(crate) fn set_update_debug_state(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
    payload: Option<UpdateDebugStatePayload>,
) -> Result<UpdateStatus, AppError> {
    if !cfg!(debug_assertions) {
        return Err(AppError::Message("update_debug_unavailable".into()));
    }

    let shared = state.inner().clone();
    *shared.pending_update.lock().unwrap() = None;

    let next = match payload {
        Some(payload) => {
            let next = build_debug_status(version_of(&app), payload)?;
            *shared.update_debug_override.lock().unwrap() = Some(next.clone());
            next
        }
        None => {
            *shared.update_debug_override.lock().unwrap() = None;
            UpdateStatus::idle(version_of(&app))
        }
    };

    Ok(emit_status(&app, &shared, next))
}

#[cfg(test)]
mod tests {
    use super::build_debug_status;
    use crate::models::UpdateDebugStatePayload;

    #[test]
    fn accepts_supported_debug_status() {
        let payload = UpdateDebugStatePayload {
            status: "available".into(),
            latest_version: Some("9.9.9-dev".into()),
            body: Some("Debug release notes".into()),
            published_at: Some("2026-04-11T00:00:00Z".into()),
            downloaded_bytes: None,
            content_length: None,
            error: None,
        };

        let next = build_debug_status("0.3.2".into(), payload).expect("debug status");

        assert_eq!(next.status, "available");
        assert_eq!(next.current_version, "0.3.2");
        assert_eq!(next.latest_version.as_deref(), Some("9.9.9-dev"));
    }

    #[test]
    fn rejects_unsupported_debug_status() {
        let payload = UpdateDebugStatePayload {
            status: "unexpected".into(),
            latest_version: None,
            body: None,
            published_at: None,
            downloaded_bytes: None,
            content_length: None,
            error: None,
        };

        let error = build_debug_status("0.3.2".into(), payload).expect_err("invalid status");

        assert_eq!(error.to_string(), "invalid_update_debug_status");
    }
}
