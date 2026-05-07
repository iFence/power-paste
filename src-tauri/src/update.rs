use std::sync::Arc;

use anyhow::Result;
use reqwest::header::{ACCEPT, USER_AGENT};
use semver::Version;
use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;

use crate::models::{
    AppError, SharedState, UpdateDebugStatePayload, UpdateStatus, UPDATE_STATUS_EVENT,
};

const GITHUB_RELEASES_API_URL: &str =
    "https://api.github.com/repos/iFence/power-paste/releases?per_page=100";
const GITHUB_RELEASES_ACCEPT: &str = "application/vnd.github+json";
const GITHUB_RELEASES_USER_AGENT: &str = "power-paste-updater";
const EMPTY_RELEASE_NOTES_TEXT: &str = "No release notes were provided for this version.";

#[derive(Clone, Copy)]
enum UpdateCheckSource {
    Startup,
    Manual,
    Recovery,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubRelease {
    tag_name: String,
    body: Option<String>,
    draft: bool,
}

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

fn parse_release_version(raw: &str) -> Option<Version> {
    Version::parse(raw.trim().trim_start_matches('v')).ok()
}

fn release_notes_section(version: &str, body: Option<&str>) -> String {
    let content = body
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(EMPTY_RELEASE_NOTES_TEXT);
    format!("## {version}\n\n{content}")
}

fn build_aggregated_release_notes(
    current_version: &str,
    latest_version: &str,
    latest_body: Option<&str>,
    releases: &[GithubRelease],
) -> Option<String> {
    let current = parse_release_version(current_version)?;
    let latest = parse_release_version(latest_version)?;
    let mut matched = releases
        .iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            let version = parse_release_version(&release.tag_name)?;
            if version <= current || version > latest {
                return None;
            }

            Some((version, release))
        })
        .collect::<Vec<_>>();

    matched.sort_by(|left, right| right.0.cmp(&left.0));

    if matched.is_empty() {
        return latest_body
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| release_notes_section(latest_version, Some(value)));
    }

    let notes = matched
        .into_iter()
        .map(|(_, release)| release_notes_section(&release.tag_name, release.body.as_deref()))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    Some(notes)
}

async fn fetch_aggregated_release_notes(
    current_version: &str,
    latest_version: &str,
    latest_body: Option<&str>,
) -> Result<Option<String>> {
    let client = reqwest::Client::new();
    let releases = client
        .get(GITHUB_RELEASES_API_URL)
        .header(ACCEPT, GITHUB_RELEASES_ACCEPT)
        .header(USER_AGENT, GITHUB_RELEASES_USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<GithubRelease>>()
        .await?;

    Ok(build_aggregated_release_notes(
        current_version,
        latest_version,
        latest_body,
        &releases,
    ))
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
        let _ = check_for_updates_inner(app, shared, UpdateCheckSource::Startup, false).await;
    });
}

pub(crate) fn spawn_manual_check(app: AppHandle, shared: Arc<SharedState>) {
    tauri::async_runtime::spawn(async move {
        let _ = check_for_updates_inner(app, shared, UpdateCheckSource::Manual, false).await;
    });
}

fn build_idle_status(app: &AppHandle) -> UpdateStatus {
    UpdateStatus {
        status: "idle".into(),
        current_version: version_of(app),
        latest_version: None,
        body: None,
        published_at: None,
        downloaded_bytes: None,
        content_length: None,
        error: None,
    }
}

async fn check_for_updates_inner(
    app: AppHandle,
    shared: Arc<SharedState>,
    source: UpdateCheckSource,
    force: bool,
) -> Result<UpdateStatus> {
    if let Some(next) = active_debug_status(&shared) {
        *shared.pending_update.lock().unwrap() = None;
        return Ok(emit_status(&app, &shared, next));
    }

    let current = current_status(&shared);
    if !force && matches!(current.status.as_str(), "checking" | "downloading") {
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
            let aggregated_body = match fetch_aggregated_release_notes(
                &version_of(&app),
                &update.version,
                update.body.as_deref(),
            )
            .await
            {
                Ok(body) => body.or_else(|| update.body.clone()),
                Err(error) => {
                    eprintln!("update release notes fetch error: {error}");
                    update.body.clone()
                }
            };
            let next = UpdateStatus {
                status: "available".into(),
                current_version: version_of(&app),
                latest_version: Some(update.version.clone()),
                body: aggregated_body,
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
            if matches!(source, UpdateCheckSource::Startup) {
                return Ok(emit_status(&app, &shared, build_idle_status(&app)));
            }

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
    check_for_updates_inner(app, state.inner().clone(), UpdateCheckSource::Manual, false)
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
        eprintln!("update install error: {error}");
        return check_for_updates_inner(app, shared, UpdateCheckSource::Recovery, true)
            .await
            .map_err(Into::into);
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
    use super::{build_aggregated_release_notes, build_debug_status, GithubRelease};
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

        let next = build_debug_status("0.3.4".into(), payload).expect("debug status");

        assert_eq!(next.status, "available");
        assert_eq!(next.current_version, "0.3.4");
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

        let error = build_debug_status("0.3.4".into(), payload).expect_err("invalid status");

        assert_eq!(error.to_string(), "invalid_update_debug_status");
    }

    #[test]
    fn aggregates_release_notes_between_current_and_latest_versions() {
        let releases = vec![
            GithubRelease {
                tag_name: "v0.5.1".into(),
                body: Some("Latest patch".into()),
                draft: false,
            },
            GithubRelease {
                tag_name: "v0.5.0".into(),
                body: Some("Minor release".into()),
                draft: false,
            },
            GithubRelease {
                tag_name: "v0.4.1".into(),
                body: Some("Hotfix".into()),
                draft: false,
            },
            GithubRelease {
                tag_name: "v0.4.0".into(),
                body: Some("Current version".into()),
                draft: false,
            },
        ];

        let notes = build_aggregated_release_notes("0.4.0", "0.5.1", None, &releases)
            .expect("aggregated notes");

        assert!(notes.contains("## v0.5.1"));
        assert!(notes.contains("## v0.5.0"));
        assert!(notes.contains("## v0.4.1"));
        assert!(!notes.contains("## v0.4.0"));
    }

    #[test]
    fn falls_back_to_latest_body_when_release_list_does_not_match() {
        let releases = vec![GithubRelease {
            tag_name: "v0.3.9".into(),
            body: Some("Older version".into()),
            draft: false,
        }];

        let notes = build_aggregated_release_notes(
            "0.4.0",
            "0.5.1",
            Some("Latest release notes"),
            &releases,
        )
        .expect("fallback notes");

        assert!(notes.contains("## 0.5.1"));
        assert!(notes.contains("Latest release notes"));
    }
}
