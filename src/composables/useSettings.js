import { computed, reactive, ref } from "vue";
import { accentColorOptions, localeOptions, themeModeOptions, translate } from "../i18n";
import {
  getAppVersion,
  getDefaultDownloadDir,
  getPlatformCapabilities as fetchPlatformCapabilities,
  getSettings as fetchSettings,
  resetSettings as resetPersistedSettings,
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
    pollingIntervalMs: 500,
    maxHistoryItems: 200,
    maxHistoryDays: 30,
    maxImageBytes: 6_000_000,
    lanTransferDownloadDir: "",
    globalShortcut: "Ctrl+Shift+V",
    ignoredApps: [],
    locale: "zh-CN",
    density: "compact",
    themeMode: "system",
    accentColor: "amber",
    tagLabels: createEmptyTagLabels(),
  });
  const recordingShortcut = ref(false);
  const openSelectKey = ref(null);
  const savingSettings = ref(false);
  const pendingSettingKey = ref("");
  const settingsSaveError = ref("");
  const startupError = ref("");
  const appVersion = ref("");
  const platformCapabilities = ref(initialPlatformCapabilities(detectedPlatform));

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

  async function syncSettings(next) {
    const defaultDownloadDir = await getDefaultDownloadDir();
    Object.assign(settings, {
      ...next,
      lanTransferDownloadDir: next.lanTransferDownloadDir || defaultDownloadDir,
      globalShortcut: normalizeShortcutValue(next.globalShortcut, detectedPlatform),
      tagLabels: normalizeTagLabels(next.tagLabels),
    });
    if (!platformCapabilities.value.supportsLaunchOnStartup) {
      settings.launchOnStartup = false;
    }
  }

  function buildSettingsPayload(sourceSettings = settings) {
    return {
      ...sourceSettings,
      globalShortcut: normalizeShortcutValue(sourceSettings.globalShortcut, detectedPlatform),
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
    localeOptions,
    openSelectKey,
    pendingSettingKey,
    platformCapabilities,
    recordingShortcut,
    refreshSettings,
    resetVisibleSettings,
    savingSettings,
    segmentedToggleStyle,
    selectedOptionLabel,
    setStartupError,
    settings,
    settingsSaveError,
    startupError,
    t,
    toggleSelect,
  };
}
