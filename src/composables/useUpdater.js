import { computed, ref } from "vue";
import {
  checkForUpdates,
  getUpdateState,
  installUpdate,
} from "../services/tauriApi";

function formatErrorMessage(error) {
  if (typeof error === "string") {
    return error;
  }
  if (error && typeof error === "object" && typeof error.message === "string") {
    return error.message;
  }
  return "";
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

  async function refreshUpdateState() {
    applyUpdateState(await getUpdateState());
  }

  async function runUpdateCheck() {
    try {
      applyUpdateState(await checkForUpdates());
    } catch (error) {
      applyUpdateState({
        status: "error",
        error: formatErrorMessage(error) || t("updateCheckFailed"),
      });
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

  return {
    canInstallUpdate,
    progressPercent,
    refreshUpdateState,
    runUpdateCheck,
    runUpdateInstall,
    statusMessage,
    updateBusy,
    updateState,
    applyUpdateState,
  };
}
