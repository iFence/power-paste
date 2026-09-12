// 局域网互传的错误码到文案 key 的映射；后端只输出稳定错误码与原始细节。
const LAN_ERROR_KEYS = {
  lan_port_in_use: "lanErrorPortInUse",
  lan_multicast_unavailable: "lanErrorMulticastUnavailable",
  lan_listener_failed: "lanErrorListenerFailed",
  lan_service_failed: "lanErrorServiceFailed",
  lan_transfer_not_running: "lanErrorNotRunning",
  lan_transfer_device_missing: "lanErrorDeviceMissing",
  lan_transfer_no_files: "lanErrorNoFiles",
  lan_transfer_file_not_found: "lanErrorFileNotFound",
  lan_transfer_request_missing: "lanErrorRequestMissing",
  lan_transfer_download_dir_missing: "lanTransferDownloadDirMissing",
  lan_transfer_download_dir_not_directory: "lanTransferDownloadDirNotDirectory",
  lan_transfer_download_dir_not_writable: "lanTransferDownloadDirNotWritable",
  pin_required: "lanErrorPinRequired",
  declined: "lanErrorDeclined",
  busy: "lanErrorBusy",
  too_many_requests: "lanErrorTooManyRequests",
  empty_payload: "lanErrorEmptyText",
};

// 取出后端错误里的稳定错误码；不是已知错误码时返回空串。
export function lanErrorCode(value) {
  const text =
    typeof value === "string"
      ? value
      : String(value?.message ?? value ?? "");
  const trimmed = text.trim();
  return Object.prototype.hasOwnProperty.call(LAN_ERROR_KEYS, trimmed)
    ? trimmed
    : "";
}

// 把错误码翻译成当前语言；未知错误回退到通用文案并保留原始细节。
export function lanErrorText(t, code, detail = "") {
  const key = LAN_ERROR_KEYS[code];
  if (key) {
    return t(key);
  }
  const extra = detail || code;
  if (!extra) {
    return t("lanErrorServiceFailed");
  }
  return t("lanErrorGeneric", { detail: extra });
}
