import { computed, ref } from "vue";
import {
  checkForUpdates,
  getUpdateState,
  installUpdate,
  setUpdateDebugState as persistUpdateDebugState,
} from "../services/tauriApi";

const UPDATE_DEBUG_STORAGE_KEY = "power-paste:update-debug-state";
const UPDATE_DEBUG_QUERY_STATUS_KEY = "updateDebug";
const UPDATE_DEBUG_QUERY_VERSION_KEY = "updateDebugVersion";
const UPDATE_DEBUG_QUERY_BODY_KEY = "updateDebugBody";
const UPDATE_DEBUG_QUERY_PUBLISHED_AT_KEY = "updateDebugPublishedAt";
const UPDATE_DEBUG_QUERY_DOWNLOADED_BYTES_KEY = "updateDebugDownloadedBytes";
const UPDATE_DEBUG_QUERY_CONTENT_LENGTH_KEY = "updateDebugContentLength";
const UPDATE_DEBUG_QUERY_ERROR_KEY = "updateDebugError";
const DEFAULT_DEBUG_VERSION = "9.9.9-dev";
const DEFAULT_DEBUG_PUBLISHED_AT = "2026-04-11T00:00:00Z";
const DEFAULT_DEBUG_BODY = [
  "## Debug Update",
  "- Preview the update badge in development.",
  "- Validate the confirmation dialog layout and release notes.",
  "- Exercise downloading and error states without a real release.",
].join("\n");

function formatErrorMessage(error) {
  const raw =
    typeof error === "string"
      ? error
      : error && typeof error === "object" && typeof error.message === "string"
        ? error.message
        : "";

  if (!raw) {
    return "";
  }

  if (
    raw.includes("https://github.com/iFence/power-paste/releases/latest/download/latest.json")
  ) {
    return "Unable to load latest.json from GitHub Releases. Confirm the tagged release is published instead of draft, and that the latest release includes the updater artifacts.";
  }

  return raw.replace("error sendingrequest", "error sending request");
}

function updateDebugEnabled() {
  return Boolean(import.meta.env.DEV);
}

function normalizeUpdateDebugStatus(rawStatus) {
  if (typeof rawStatus !== "string") {
    return null;
  }

  const status = rawStatus.trim().toLowerCase();
  if (!status) {
    return null;
  }

  if (["clear", "off", "none"].includes(status)) {
    return "clear";
  }

  if (status === "up-to-date" || status === "uptodate") {
    return "up_to_date";
  }

  if (
    ["idle", "checking", "available", "downloading", "downloaded", "up_to_date", "error"].includes(
      status,
    )
  ) {
    return status;
  }

  return null;
}

function parseOptionalNumber(rawValue) {
  if (rawValue == null || rawValue === "") {
    return undefined;
  }

  const next = Number(rawValue);
  return Number.isFinite(next) ? next : undefined;
}

function buildUpdateDebugPayload(status, overrides = {}) {
  const normalizedStatus = normalizeUpdateDebugStatus(status);
  if (!normalizedStatus || normalizedStatus === "clear") {
    return null;
  }

  const payload = {
    status: normalizedStatus,
    latestVersion: null,
    body: null,
    publishedAt: null,
    downloadedBytes: null,
    contentLength: null,
    error: null,
  };

  if (["available", "downloading", "downloaded"].includes(normalizedStatus)) {
    payload.latestVersion = DEFAULT_DEBUG_VERSION;
    payload.body = DEFAULT_DEBUG_BODY;
    payload.publishedAt = DEFAULT_DEBUG_PUBLISHED_AT;
  }

  if (normalizedStatus === "downloading") {
    payload.downloadedBytes = 36;
    payload.contentLength = 100;
  }

  if (normalizedStatus === "downloaded") {
    payload.downloadedBytes = 100;
    payload.contentLength = 100;
  }

  return {
    ...payload,
    ...(overrides.latestVersion !== undefined ? { latestVersion: overrides.latestVersion } : {}),
    ...(overrides.body !== undefined ? { body: overrides.body } : {}),
    ...(overrides.publishedAt !== undefined ? { publishedAt: overrides.publishedAt } : {}),
    ...(overrides.downloadedBytes !== undefined
      ? { downloadedBytes: overrides.downloadedBytes }
      : {}),
    ...(overrides.contentLength !== undefined ? { contentLength: overrides.contentLength } : {}),
    ...(overrides.error !== undefined ? { error: overrides.error } : {}),
  };
}

function readStoredUpdateDebugPayload() {
  if (!updateDebugEnabled()) {
    return undefined;
  }

  try {
    const raw = window.localStorage.getItem(UPDATE_DEBUG_STORAGE_KEY);
    if (!raw) {
      return undefined;
    }

    const parsed = JSON.parse(raw);
    const payload = buildUpdateDebugPayload(parsed?.status, parsed ?? {});
    if (!payload) {
      window.localStorage.removeItem(UPDATE_DEBUG_STORAGE_KEY);
      return undefined;
    }

    return payload;
  } catch {
    window.localStorage.removeItem(UPDATE_DEBUG_STORAGE_KEY);
    return undefined;
  }
}

function writeStoredUpdateDebugPayload(payload) {
  if (!updateDebugEnabled()) {
    return;
  }

  if (!payload) {
    window.localStorage.removeItem(UPDATE_DEBUG_STORAGE_KEY);
    return;
  }

  window.localStorage.setItem(UPDATE_DEBUG_STORAGE_KEY, JSON.stringify(payload));
}

function readUpdateDebugQueryConfig() {
  if (!updateDebugEnabled()) {
    return { hasConfig: false };
  }

  const params = new URLSearchParams(window.location.search);
  const rawStatus = params.get(UPDATE_DEBUG_QUERY_STATUS_KEY);
  if (rawStatus == null) {
    return { hasConfig: false };
  }

  const normalizedStatus = normalizeUpdateDebugStatus(rawStatus);
  if (!normalizedStatus) {
    return { hasConfig: false };
  }

  if (normalizedStatus === "clear") {
    return { hasConfig: true, payload: null };
  }

  return {
    hasConfig: true,
    payload: buildUpdateDebugPayload(normalizedStatus, {
      latestVersion: params.get(UPDATE_DEBUG_QUERY_VERSION_KEY) ?? undefined,
      body: params.get(UPDATE_DEBUG_QUERY_BODY_KEY) ?? undefined,
      publishedAt: params.get(UPDATE_DEBUG_QUERY_PUBLISHED_AT_KEY) ?? undefined,
      downloadedBytes: parseOptionalNumber(params.get(UPDATE_DEBUG_QUERY_DOWNLOADED_BYTES_KEY)),
      contentLength: parseOptionalNumber(params.get(UPDATE_DEBUG_QUERY_CONTENT_LENGTH_KEY)),
      error: params.get(UPDATE_DEBUG_QUERY_ERROR_KEY) ?? undefined,
    }),
  };
}

export function useUpdater({ t }) {
  const updateState = ref({
    status: "idle",
    currentVersion: "",
    latestVersion: null,
    body: null,
    publishedAt: null,
    downloadedBytes: null,
    contentLength: null,
    error: null,
  });
  const updateDebugStatus = ref(null);
  const isUpdateDebugEnabled = updateDebugEnabled();
  const updateBusy = computed(() =>
    ["checking", "downloading"].includes(updateState.value.status),
  );
  const canInstallUpdate = computed(() => updateState.value.status === "available");
  const progressPercent = computed(() => {
    const downloaded = Number(updateState.value.downloadedBytes ?? 0);
    const total = Number(updateState.value.contentLength ?? 0);
    if (!downloaded || !total || total <= 0) {
      return null;
    }
    return Math.max(0, Math.min(100, Math.round((downloaded / total) * 100)));
  });
  const statusMessage = computed(() => {
    const latestVersion = updateState.value.latestVersion;
    switch (updateState.value.status) {
      case "checking":
        return t("checkingForUpdates");
      case "available":
        return latestVersion
          ? t("updateAvailableVersion", { version: latestVersion })
          : t("updateAvailable");
      case "downloading":
        if (progressPercent.value != null) {
          return t("downloadingUpdateProgress", { percent: progressPercent.value });
        }
        return t("downloadingUpdate");
      case "downloaded":
        return t("updateReadyToInstall");
      case "up_to_date":
        return t("upToDate");
      case "error":
        return updateState.value.error || t("updateCheckFailed");
      default:
        return t("updateIdle");
    }
  });

  function applyUpdateState(next) {
    updateState.value = {
      ...updateState.value,
      ...next,
    };
  }

  async function applyUpdateDebugOverride(payload) {
    const nextState = await persistUpdateDebugState(payload);
    updateDebugStatus.value = payload?.status ?? null;
    applyUpdateState(nextState);
    return nextState;
  }

  async function syncUpdateDebugOverride() {
    if (!isUpdateDebugEnabled) {
      return null;
    }

    const queryConfig = readUpdateDebugQueryConfig();
    let payload;

    if (queryConfig.hasConfig) {
      payload = queryConfig.payload;
      writeStoredUpdateDebugPayload(payload);
    } else {
      payload = readStoredUpdateDebugPayload();
    }

    if (payload === undefined) {
      updateDebugStatus.value = null;
      return null;
    }

    return applyUpdateDebugOverride(payload);
  }

  async function refreshUpdateState() {
    const debugState = await syncUpdateDebugOverride();
    if (debugState) {
      return debugState;
    }

    const nextState = await getUpdateState();
    applyUpdateState(nextState);
    return nextState;
  }

  async function runUpdateCheck() {
    try {
      const debugState = await syncUpdateDebugOverride();
      if (debugState) {
        return debugState;
      }

      const nextState = await checkForUpdates();
      applyUpdateState(nextState);
      return nextState;
    } catch (error) {
      applyUpdateState({
        status: "error",
        error: formatErrorMessage(error) || t("updateCheckFailed"),
      });
      return updateState.value;
    }
  }

  async function runUpdateInstall() {
    try {
      applyUpdateState(await installUpdate());
    } catch (error) {
      applyUpdateState({
        status: "error",
        error: formatErrorMessage(error) || t("updateInstallFailed"),
      });
    }
  }

  async function setUpdateDebugStatus(status) {
    if (!isUpdateDebugEnabled) {
      return updateState.value;
    }

    const payload = buildUpdateDebugPayload(status);
    if (!payload) {
      return updateState.value;
    }

    writeStoredUpdateDebugPayload(payload);
    return applyUpdateDebugOverride(payload);
  }

  async function clearUpdateDebugStatus() {
    if (!isUpdateDebugEnabled) {
      return updateState.value;
    }

    writeStoredUpdateDebugPayload(null);
    return applyUpdateDebugOverride(null);
  }

  return {
    canInstallUpdate,
    clearUpdateDebugStatus,
    progressPercent,
    refreshUpdateState,
    runUpdateCheck,
    runUpdateInstall,
    setUpdateDebugStatus,
    statusMessage,
    updateBusy,
    updateDebugEnabled: isUpdateDebugEnabled,
    updateDebugStatus,
    updateState,
    applyUpdateState,
  };
}
