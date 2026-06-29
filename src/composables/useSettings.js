import { computed, reactive, ref } from "vue";
import { accentColorOptions, localeOptions, themeModeOptions, translate } from "../i18n";
import {
  getAppVersion,
  getDefaultDownloadDir,
  getWebdavSyncState,
  getPlatformCapabilities as fetchPlatformCapabilities,
  getSettings as fetchSettings,
  getShortcutStatus,
  retryShortcutRegistration as retryPersistedShortcutRegistration,
  resetSettings as resetPersistedSettings,
  clearWebdavCredential as removeWebdavCredential,
  syncWebdavNow,
  testWebdavSync,
  updateWebdavCredential as saveWebdavCredential,
  updateSettings as persistSettings,
} from "../services/tauriApi";
import { normalizeShortcutValue } from "../utils/shortcut";
import { createEmptyTagLabels, normalizeTagLabels } from "../utils/constants";

function detectClientPlatform() {
  const userAgent = window.navigator.userAgent.toLowerCase();

  if (userAgent.includes("mac os x") || userAgent.includes("macintosh")) {
    return "macos";
  }

  if (userAgent.includes("windows")) {
    return "windows";
  }

  if (userAgent.includes("linux")) {
    return "linux";
  }

  return "unknown";
}

function extractErrorCode(error) {
  if (typeof error === "string") {
    return error;
  }
  if (error && typeof error === "object" && typeof error.message === "string") {
    return error.message;
  }
  return "";
}

function initialPlatformCapabilities(platform) {
  const isWindows = platform === "windows";
  const isMacos = platform === "macos";
  const isLinux = platform === "linux";

  return {
    platform,
    supportsClipboardRead: true,
    supportsClipboardWatch: true,
    supportsTextWrite: true,
    supportsHtmlWrite: true,
    supportsImageWrite: true,
    supportsDirectPaste: isWindows || isMacos,
    supportsLaunchOnStartup: isWindows || isMacos || isLinux,
    supportsHardwareAccelerationToggle: isWindows,
    supportsMixedReplay: isWindows,
    preferredClipboardBackend: isWindows
      ? "plugin+native-fallback"
      : isMacos
        ? "plugin-preferred"
        : "plugin-only",
    clipboardWriteStrategy: isWindows
      ? "plugin-first-with-native-fallback"
      : isMacos
        ? "plugin-first-with-mixed-degradation"
        : "plugin-only",
    directPasteStrategy: isWindows || isMacos
      ? "simulated-native-shortcut"
      : isLinux
        ? "linux-tooling-runtime-check"
        : "unsupported",
    mixedReplayStrategy: isWindows
      ? "target-aware-segmented-replay"
      : isMacos
        ? "plugin-degraded-single-payload"
        : isLinux
          ? "plugin-degraded-single-payload"
          : "unsupported",
  };
}

function directPasteUnavailableMessage(capabilities, t) {
  if (!capabilities || capabilities.supportsDirectPaste) {
    return t("unsupportedDirectPaste");
  }

  if (capabilities.platform === "linux") {
    if (capabilities.directPasteStrategy === "wayland-wtype-required") {
      return t("linuxWaylandToolsMissing");
    }
    if (capabilities.directPasteStrategy === "x11-tooling-required") {
      return t("linuxX11ToolsMissing");
    }
  }

  return t("unsupportedDirectPaste");
}

export function useSettings() {
  const detectedPlatform = detectClientPlatform();
  const settings = reactive({
    debugEnabled: false,
    soundEnabled: true,
    launchOnStartup: false,
    hardwareAccelerationEnabled: true,
    pollingIntervalMs: 500,
    maxHistoryItems: 200,
    maxHistoryDays: 30,
    maxImageBytes: 6_000_000,
    copyStatsEnabled: false,
    pasteStatsEnabled: false,
    lanTransferDownloadDir: "",
    globalShortcut: "Ctrl+Shift+V",
    quickPasteShortcut: "Ctrl+`",
    searchShortcut: "Ctrl+F",
    filterShortcut: "Ctrl+Tab",
    ignoredApps: [],
    locale: "zh-CN",
    density: "compact",
    themeMode: "system",
    accentColor: "amber",
    tagLabels: createEmptyTagLabels(),
    webdavSync: {
      enabled: false,
      autoSync: true,
      serverUrl: "",
      username: "",
      remoteDir: "power-paste",
    },
  });
  const recordingShortcut = ref(false);
  const openSelectKey = ref(null);
  const savingSettings = ref(false);
  const pendingSettingKey = ref("");
  const settingsSaveError = ref("");
  const startupError = ref("");
  const appVersion = ref("");
  const shortcutRetrying = ref(false);
  const shortcutStatus = ref({
    globalShortcutRegistered: false,
    quickPasteShortcutRegistered: false,
    issues: [],
  });
  const platformCapabilities = ref(initialPlatformCapabilities(detectedPlatform));
  const webdavSyncStatus = ref({
    status: "idle",
    lastSyncAt: null,
    error: null,
    changedCount: 0,
  });
  const webdavPasswordDraft = ref("");
  const webdavCredentialSaved = ref(false);

  const currentLocale = computed(() => settings.locale || "zh-CN");
  const currentDensity = computed(() => settings.density || "compact");
  const currentThemeMode = computed(() => settings.themeMode || "system");
  const currentAccentColor = computed(() => settings.accentColor || "amber");
  const currentThemeModeOptions = computed(
    () => themeModeOptions[currentLocale.value] || themeModeOptions["en-US"],
  );
  const currentAccentColorOptions = computed(
    () => accentColorOptions[currentLocale.value] || accentColorOptions["en-US"],
  );
  const canToggleLaunchOnStartup = computed(
    () => platformCapabilities.value.supportsLaunchOnStartup,
  );
  const shortcutIssues = computed(() => shortcutStatus.value?.issues || []);
  const hasShortcutIssues = computed(() => shortcutIssues.value.length > 0);
  const shortcutWarningMessage = computed(() => {
    const issue = shortcutIssues.value[0];
    if (!issue) {
      return "";
    }

    const label =
      issue.key === "quickPasteShortcut" ? t("quickPasteShortcut") : t("globalShortcut");
    return t("shortcutConflictMessage", {
      name: label,
      shortcut: issue.shortcut || label,
    });
  });

  function t(key, params) {
    return translate(currentLocale.value, key, params);
  }

  function segmentedToggleStyle(activeIndex, optionCount) {
    return {
      "--toggle-index": String(activeIndex),
      "--toggle-count": String(optionCount),
    };
  }

  function selectedOptionLabel(options, value) {
    return options.find((option) => option.value === value)?.label ?? "";
  }

  function toggleSelect(key) {
    openSelectKey.value = openSelectKey.value === key ? null : key;
  }

  function closeSelect() {
    openSelectKey.value = null;
  }

  function formatErrorMessage(error, fallbackKey = "saveSettingsFailed") {
    const code = extractErrorCode(error);
    if (code === "linux_x11_tools_missing") {
      return t("linuxX11ToolsMissing");
    }
    if (code === "linux_wayland_tools_missing") {
      return t("linuxWaylandToolsMissing");
    }
    if (code === "unsupported_launch_on_startup") {
      return t("unsupportedLaunchOnStartup");
    }
    if (code === "lan_transfer_download_dir_missing") {
      return t("lanTransferDownloadDirMissing");
    }
    if (code === "lan_transfer_download_dir_not_directory") {
      return t("lanTransferDownloadDirNotDirectory");
    }
    if (code.includes("lan_transfer_download_dir_not_writable")) {
      return t("lanTransferDownloadDirNotWritable");
    }
    if (code === "unsupported_clipboard_write") {
      return t("unsupportedClipboardWrite");
    }
    if (code === "unsupported_direct_paste") {
      return t("unsupportedDirectPaste");
    }
    if (code === "duplicate_shortcut") {
      return t("duplicateShortcut");
    }
    if (
      code.startsWith("invalid_global_shortcut") ||
      code.startsWith("invalid_quick_paste_shortcut")
    ) {
      return t("invalidShortcut");
    }
    if (code.startsWith("shortcut_registration_failed")) {
      return t("shortcutRegistrationFailed");
    }
    if (code === "webdav_settings_incomplete") {
      return t("webdavSettingsIncomplete");
    }
    if (code.includes("webdav_credential_missing")) {
      return t("webdavCredentialMissing");
    }
    if (code.includes("webdav_endpoint_gone") || code.includes("410 Gone")) {
      return t("webdavEndpointGone");
    }
    if (code.startsWith("webdav_") && code.includes("401")) {
      return t("webdavUnauthorized");
    }
    if (code.startsWith("webdav_") && code.includes("403")) {
      return t("webdavForbidden");
    }
    if (code.startsWith("webdav_") && code.includes("404")) {
      return t("webdavNotFound");
    }
    if (code.startsWith("webdav_") && code.includes("405")) {
      return t("webdavMethodNotAllowed");
    }
    if (code.startsWith("webdav_") && code.includes("503")) {
      return t("webdavServiceUnavailable");
    }
    if (code.includes("webdav_connection_failed")) {
      if (code.includes("401")) {
        return t("webdavUnauthorized");
      }
      if (code.includes("403")) {
        return t("webdavForbidden");
      }
      if (code.includes("404")) {
        return t("webdavNotFound");
      }
      if (code.includes("405")) {
        return t("webdavMethodNotAllowed");
      }
      if (code.includes("503")) {
        return t("webdavServiceUnavailable");
      }
      return t("webdavConnectionFailed");
    }
    if (code.includes("webdav_item_delete_failed")) {
      return t("webdavRemoteCleanupFailed");
    }
    if (code.includes("webdav_manifest_put_failed")) {
      return t("webdavManifestSaveFailed");
    }
    if (code.includes("webdav_manifest_fetch_failed")) {
      return t("webdavManifestFetchFailed");
    }
    if (code.includes("webdav_item_put_failed")) {
      return t("webdavItemUploadFailed");
    }
    if (code.includes("webdav_item_fetch_failed")) {
      return t("webdavItemDownloadFailed");
    }
    if (code.includes("webdav_mkcol_failed")) {
      return t("webdavFolderCreateFailed");
    }
    if (code.startsWith("webdav_")) {
      return t("webdavSyncFailed");
    }
    if (typeof error === "string") {
      return error;
    }
    if (error && typeof error === "object") {
      if (typeof error.message === "string") {
        return error.message;
      }
      if ("toString" in error && typeof error.toString === "function") {
        const text = error.toString();
        if (text && text !== "[object Object]") {
          return text;
        }
      }
    }
    return t(fallbackKey);
  }

  function setStartupError(error) {
    startupError.value = formatErrorMessage(error, "startupLoadFailed");
  }

  function clearStartupError() {
    startupError.value = "";
  }

  function beginShortcutRecording() {
    recordingShortcut.value = true;
  }

  function endShortcutRecording() {
    recordingShortcut.value = false;
  }

  async function loadAppVersion() {
    appVersion.value = (await getAppVersion()) || "";
  }

  async function loadPlatformCapabilities() {
    platformCapabilities.value = await fetchPlatformCapabilities();
  }

  async function refreshSettings() {
    const next = await fetchSettings();
    await syncSettings(next);
  }

  async function loadShortcutStatus() {
    shortcutStatus.value = await getShortcutStatus();
  }

  async function refreshWebdavSyncState() {
    webdavSyncStatus.value = await getWebdavSyncState();
  }

  async function syncSettings(next) {
    const defaultDownloadDir = await getDefaultDownloadDir();
    Object.assign(settings, {
      ...next,
      lanTransferDownloadDir: next.lanTransferDownloadDir || defaultDownloadDir,
      globalShortcut: normalizeShortcutValue(next.globalShortcut, detectedPlatform),
      quickPasteShortcut: normalizeShortcutValue(next.quickPasteShortcut, detectedPlatform),
      searchShortcut: normalizeShortcutValue(next.searchShortcut, detectedPlatform),
      filterShortcut: normalizeShortcutValue(next.filterShortcut, detectedPlatform),
      tagLabels: normalizeTagLabels(next.tagLabels),
      webdavSync: {
        enabled: Boolean(next.webdavSync?.enabled),
        autoSync: next.webdavSync?.autoSync !== false,
        credentialSaved: Boolean(next.webdavSync?.credentialSaved),
        serverUrl: next.webdavSync?.serverUrl || "",
        username: next.webdavSync?.username || "",
        remoteDir: next.webdavSync?.remoteDir || "power-paste",
      },
    });
    webdavCredentialSaved.value = Boolean(next.webdavSync?.credentialSaved);
    if (!platformCapabilities.value.supportsLaunchOnStartup) {
      settings.launchOnStartup = false;
    }
  }

  function buildSettingsPayload(sourceSettings = settings) {
    return {
      ...sourceSettings,
      globalShortcut: normalizeShortcutValue(sourceSettings.globalShortcut, detectedPlatform),
      quickPasteShortcut: normalizeShortcutValue(
        sourceSettings.quickPasteShortcut,
        detectedPlatform,
      ),
      searchShortcut: normalizeShortcutValue(sourceSettings.searchShortcut, detectedPlatform),
      filterShortcut: normalizeShortcutValue(sourceSettings.filterShortcut, detectedPlatform),
      tagLabels: normalizeTagLabels(sourceSettings.tagLabels),
      launchOnStartup: platformCapabilities.value.supportsLaunchOnStartup
        ? sourceSettings.launchOnStartup
        : false,
    };
  }

  async function applySettingPatch(patch, key = "") {
    if (savingSettings.value) {
      return;
    }

    const previous = { ...settings };
    const payload = buildSettingsPayload({
      ...settings,
      ...patch,
    });
    settingsSaveError.value = "";
    savingSettings.value = true;
    pendingSettingKey.value = key;
    closeSelect();
    Object.assign(settings, payload);

    try {
      await persistSettings(payload);
      Object.assign(settings, payload);
    } catch (error) {
      Object.assign(settings, previous);
      settingsSaveError.value = formatErrorMessage(error);
      console.error("Failed to save settings", error);
    } finally {
      pendingSettingKey.value = "";
      savingSettings.value = false;
    }
  }

  async function applyWebdavSyncPatch(patch, key = "webdavSync") {
    await applySettingPatch(
      {
        webdavSync: {
          ...settings.webdavSync,
          ...patch,
        },
      },
      key,
    );
  }

  async function saveWebdavPassword(password = webdavPasswordDraft.value) {
    if (!password) {
      return;
    }
    await saveWebdavCredential(password);
    webdavPasswordDraft.value = "";
    webdavCredentialSaved.value = true;
    await applyWebdavSyncPatch({ credentialSaved: true }, "webdavSync.credentialSaved");
  }

  async function clearWebdavPassword() {
    await removeWebdavCredential();
    webdavPasswordDraft.value = "";
    webdavCredentialSaved.value = false;
    await applyWebdavSyncPatch({ credentialSaved: false }, "webdavSync.credentialSaved");
  }

  async function runWebdavTest() {
    savingSettings.value = true;
    pendingSettingKey.value = "webdavSync.test";
    try {
      webdavSyncStatus.value = await testWebdavSync();
    } catch (error) {
      const message = formatErrorMessage(error, "webdavSyncFailed");
      webdavSyncStatus.value = {
        ...webdavSyncStatus.value,
        status: "error",
        error: message,
      };
    } finally {
      pendingSettingKey.value = "";
      savingSettings.value = false;
    }
  }

  async function runWebdavSyncNow() {
    savingSettings.value = true;
    pendingSettingKey.value = "webdavSync.now";
    try {
      webdavSyncStatus.value = await syncWebdavNow();
    } catch (error) {
      const message = formatErrorMessage(error, "webdavSyncFailed");
      webdavSyncStatus.value = {
        ...webdavSyncStatus.value,
        status: "error",
        error: message,
      };
    } finally {
      pendingSettingKey.value = "";
      savingSettings.value = false;
    }
  }

  function applyWebdavSyncStatus(status) {
    if (status) {
      webdavSyncStatus.value = status;
    }
  }

  function applyShortcutStatus(status) {
    if (status) {
      shortcutStatus.value = status;
    }
  }

  async function retryShortcutRegistration() {
    if (shortcutRetrying.value) {
      return;
    }

    shortcutRetrying.value = true;
    settingsSaveError.value = "";
    try {
      shortcutStatus.value = await retryPersistedShortcutRegistration();
    } catch (error) {
      settingsSaveError.value = formatErrorMessage(error);
      console.error("Failed to retry shortcut registration", error);
    } finally {
      shortcutRetrying.value = false;
    }
  }

  async function resetVisibleSettings() {
    if (savingSettings.value) {
      return;
    }

    settingsSaveError.value = "";
    savingSettings.value = true;
    pendingSettingKey.value = "reset";
    closeSelect();

    try {
      const next = await resetPersistedSettings();
      await syncSettings(next);
    } catch (error) {
      settingsSaveError.value = formatErrorMessage(error);
      console.error("Failed to reset settings", error);
    } finally {
      pendingSettingKey.value = "";
      savingSettings.value = false;
    }
  }

  return {
    applySettingPatch,
    applyShortcutStatus,
    applyWebdavSyncPatch,
    applyWebdavSyncStatus,
    appVersion,
    beginShortcutRecording,
    canToggleLaunchOnStartup,
    clearStartupError,
    closeSelect,
    currentAccentColor,
    currentAccentColorOptions,
    currentDensity,
    directPasteUnavailableMessage,
    currentLocale,
    currentThemeMode,
    currentThemeModeOptions,
    endShortcutRecording,
    loadAppVersion,
    loadPlatformCapabilities,
    loadShortcutStatus,
    localeOptions,
    openSelectKey,
    pendingSettingKey,
    platformCapabilities,
    recordingShortcut,
    refreshSettings,
    refreshWebdavSyncState,
    resetVisibleSettings,
    retryShortcutRegistration,
    runWebdavSyncNow,
    runWebdavTest,
    saveWebdavPassword,
    clearWebdavPassword,
    savingSettings,
    segmentedToggleStyle,
    selectedOptionLabel,
    setStartupError,
    settings,
    settingsSaveError,
    shortcutRetrying,
    shortcutStatus,
    shortcutWarningMessage,
    hasShortcutIssues,
    startupError,
    t,
    toggleSelect,
    webdavCredentialSaved,
    webdavPasswordDraft,
    webdavSyncStatus,
  };
}
