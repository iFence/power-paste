import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export function getAppVersion() {
  return getVersion();
}

export function onHistoryUpdated(handler) {
  return listen("history-updated", handler);
}

export function onUpdateStatus(handler) {
  return listen("update-status", handler);
}

export function getHistory(payload) {
  return invoke("get_history", payload);
}

export function getSettings() {
  return invoke("get_settings");
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

export function updateSettings(payload) {
  return invoke("update_settings", { payload });
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
