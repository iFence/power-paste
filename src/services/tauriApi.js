import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export function getAppVersion() {
  return getVersion();
}

export function onHistoryUpdated(handler) {
  return listen("history-updated", handler);
}

export function onCopySound(handler) {
  return listen("copy-sound", handler);
}

export function onUpdateStatus(handler) {
  return listen("update-status", handler);
}

export function onLanReceiverStatus(handler) {
  return listen("lan-receiver-status", handler);
}

export function onWebdavSyncStatus(handler) {
  return listen("webdav-sync-status", handler);
}

export function getHistory(payload) {
  return invoke("get_history", { payload });
}

export function getSettings() {
  return invoke("get_settings");
}

export function getDefaultDownloadDir() {
  return invoke("get_default_download_dir");
}

export function getPlatformCapabilities() {
  return invoke("get_platform_capabilities");
}

export function getUpdateState() {
  return invoke("get_update_state");
}

export function checkForUpdates() {
  return invoke("check_for_updates");
}

export function installUpdate() {
  return invoke("install_update");
}

export function setUpdateDebugState(payload) {
  return invoke("set_update_debug_state", { payload });
}

export function updateSettings(payload) {
  return invoke("update_settings", { payload });
}

export function saveMainPanelSize(payload) {
  return invoke("save_main_panel_size", { payload });
}

export function resetSettings() {
  return invoke("reset_settings");
}

export function getWebdavSyncState() {
  return invoke("get_webdav_sync_state");
}

export function updateWebdavCredential(password) {
  return invoke("update_webdav_credential", { payload: { password } });
}

export function clearWebdavCredential() {
  return invoke("clear_webdav_credential");
}

export function testWebdavSync() {
  return invoke("test_webdav_sync");
}

export function syncWebdavNow() {
  return invoke("sync_webdav_now");
}

export function togglePin(id) {
  return invoke("toggle_pin", { id });
}

export function toggleFavorite(id) {
  return invoke("toggle_favorite", { id });
}

export function deleteItem(id) {
  return invoke("delete_item", { id });
}

export function updateTextItem(id, text) {
  return invoke("update_text_item", { id, text });
}

export function updateItemTags(id, tagColors) {
  return invoke("update_item_tags", { id, tagColors });
}

export function clearHistory() {
  return invoke("clear_history");
}

export function copyItem(id) {
  return invoke("copy_item", { id });
}

export function pasteItem(id) {
  return invoke("paste_item", { id });
}

export function openExternalUrl(url) {
  return invoke("open_external_url", { url });
}

export function startLanReceiver() {
  return invoke("start_lan_receiver");
}

export function stopLanReceiver() {
  return invoke("stop_lan_receiver");
}

export function getLanReceiverState() {
  return invoke("get_lan_receiver_state");
}

export function sendLanTransferText(text) {
  return invoke("send_lan_transfer_text", { text });
}

export function sendLanTransferFile(fileName, mimeType, bytes) {
  return invoke("send_lan_transfer_file", {
    fileName,
    mimeType,
    bytes,
  });
}

export function openLanTransferFile(id) {
  return invoke("open_lan_transfer_file", { id });
}

export function revealLanTransferFile(id) {
  return invoke("reveal_lan_transfer_file", { id });
}
