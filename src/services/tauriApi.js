import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { startDrag } from "@crabnebula/tauri-plugin-drag";

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

export function onLanTransferState(handler) {
  return listen("lan-transfer-state", handler);
}

export function onWebdavSyncStatus(handler) {
  return listen("webdav-sync-status", handler);
}

export function onQuickPasteStarted(handler) {
  return listen("quick-paste-started", handler);
}

export function onOpenSettings(handler) {
  return listen("open-settings", handler);
}

export function onPanelShown(handler) {
  return listen("panel-shown", handler);
}

export function onShortcutStatusUpdated(handler) {
  return listen("shortcut-status-updated", handler);
}

export function getHistory(payload) {
  return invoke("get_history", { payload });
}

export function getSettings() {
  return invoke("get_settings");
}

export function getShortcutStatus() {
  return invoke("get_shortcut_status");
}

export function getDefaultDownloadDir() {
  return invoke("get_default_download_dir");
}

export function getPlatformCapabilities() {
  return invoke("get_platform_capabilities");
}
export function listInstalledApps() {
  return invoke("list_installed_apps");
}

export function getInstalledAppIcon(payload) {
  return invoke("get_installed_app_icon", payload);
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

export function retryShortcutRegistration() {
  return invoke("retry_shortcut_registration");
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

export function prepareImageDragFile(id) {
  return invoke("prepare_image_drag_file", { id });
}

export function startNativeFileDrag(paths, iconPath) {
  return startDrag({
    item: paths,
    icon: iconPath || paths[0],
    mode: "copy",
  });
}

export function openExternalUrl(url) {
  return invoke("open_external_url", { url });
}

export function startLanTransfer() {
  return invoke("start_lan_transfer");
}

export function stopLanTransfer() {
  return invoke("stop_lan_transfer");
}

export function getLanTransferState() {
  return invoke("get_lan_transfer_state");
}

export function refreshLanDevices() {
  return invoke("refresh_lan_devices");
}

export function listLanSubnets() {
  return invoke("list_lan_subnets");
}

export function scanLanSubnets(subnets, port = null) {
  return invoke("scan_lan_subnets", { subnets, port });
}

export function cancelLanScan() {
  return invoke("cancel_lan_scan");
}

export function addLanDevice(host, port) {
  return invoke("add_lan_device", { host, port: port || null });
}

export function sendLanFiles(fingerprint, paths, pin = null) {
  return invoke("send_lan_files", { fingerprint, paths, pin });
}

export function sendLanText(fingerprint, text, pin = null) {
  return invoke("send_lan_text", { fingerprint, text, pin });
}

export function inspectLanSelection(paths) {
  return invoke("inspect_lan_selection", { paths });
}

export function readLanClipboard() {
  return invoke("read_lan_clipboard");
}

export function sendLanItems(fingerprint, items, pin = null) {
  return invoke("send_lan_items", { fingerprint, items, pin });
}

export function respondLanRequest(requestId, decision) {
  return invoke("respond_lan_request", { requestId, decision });
}

export function cancelLanTransfer(transferId) {
  return invoke("cancel_lan_transfer", { transferId });
}

export function setLanWebMode(mode, paths = []) {
  return invoke("set_lan_web_mode", { mode, paths });
}

export function openLanReceivedFile(id) {
  return invoke("open_lan_received_file", { id });
}

export function revealLanReceivedFile(id) {
  return invoke("reveal_lan_received_file", { id });
}

export function removeLanTrustedDevice(fingerprint) {
  return invoke("remove_lan_trusted_device", { fingerprint });
}
