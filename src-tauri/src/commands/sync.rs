use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::{
    models::{AppError, SharedState, WebdavCredentialPayload, WebdavSyncStatusDto},
    sync::{
        clear_webdav_credential as execute_clear_webdav_credential,
        current_webdav_sync_state, sync_webdav_now as execute_sync_webdav_now,
        test_webdav_sync as execute_test_webdav_sync,
        update_webdav_credential as execute_update_webdav_credential,
    },
};

// 获取 WebDAV 同步状态，用于设置页展示最近同步结果。
#[tauri::command]
pub(crate) fn get_webdav_sync_state(
    state: State<'_, Arc<SharedState>>,
) -> Result<WebdavSyncStatusDto, AppError> {
    current_webdav_sync_state(state.inner())
}

// 保存 WebDAV 密码到系统凭据存储。
#[tauri::command]
pub(crate) fn update_webdav_credential(
    state: State<'_, Arc<SharedState>>,
    payload: WebdavCredentialPayload,
) -> Result<(), AppError> {
    execute_update_webdav_credential(state.inner(), payload)
}

// 清除系统凭据存储中的 WebDAV 密码。
#[tauri::command]
pub(crate) fn clear_webdav_credential(state: State<'_, Arc<SharedState>>) -> Result<(), AppError> {
    execute_clear_webdav_credential(state.inner())
}

// 测试当前 WebDAV 配置和系统凭据是否可访问。
#[tauri::command]
pub(crate) async fn test_webdav_sync(
    state: State<'_, Arc<SharedState>>,
) -> Result<WebdavSyncStatusDto, AppError> {
    execute_test_webdav_sync(state.inner().clone()).await
}

// 立即执行一次 WebDAV 双向同步。
#[tauri::command]
pub(crate) async fn sync_webdav_now(
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<WebdavSyncStatusDto, AppError> {
    execute_sync_webdav_now(app, state.inner().clone()).await
}
