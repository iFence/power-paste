use std::sync::Arc;

use anyhow::Result;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;

use crate::models::{AppError, SharedState, UpdateStatus, UPDATE_STATUS_EVENT};

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

pub(crate) fn spawn_startup_check(app: AppHandle, shared: Arc<SharedState>) {
    tauri::async_runtime::spawn(async move {
        let auto_check = shared.settings.lock().unwrap().auto_check_updates;
        if auto_check {
            let _ = check_for_updates_inner(app, shared).await;
        }
    });
}

async fn check_for_updates_inner(app: AppHandle, shared: Arc<SharedState>) -> Result<UpdateStatus> {
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

    match app.updater()?.check().await {
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

    let installed = emit_status(
        &app,
        &shared,
        UpdateStatus {
            status: "downloaded".into(),
            current_version: version_of(&app),
            latest_version: next.latest_version,
            body: next.body,
            published_at: next.published_at,
            downloaded_bytes: next.downloaded_bytes,
            content_length: next.content_length,
            error: None,
        },
    );

    #[cfg(not(windows))]
    app.restart();

    Ok(installed)
}
