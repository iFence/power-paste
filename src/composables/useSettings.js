import { computed, reactive, ref } from "vue";
import { accentColorOptions, localeOptions, themeModeOptions, translate } from "../i18n";
import {
  getAppVersion,
  getPlatformCapabilities as fetchPlatformCapabilities,
  getSettings as fetchSettings,
  updateSettings as persistSettings,
} from "../services/tauriApi";
import { normalizeShortcutKey } from "../utils/shortcut";

function extractErrorCode(error) {
  if (typeof error === "string") {
    return error;
  }
  if (error && typeof error === "object" && typeof error.message === "string") {
    return error.message;
  }
  return "";
}

export function useSettings() {
  const settings = reactive({
    debugEnabled: false,
    launchOnStartup: false,
    autoCheckUpdates: true,
    pollingIntervalMs: 500,
    maxHistoryItems: 200,
    maxImageBytes: 6_000_000,
    globalShortcut: "Ctrl+Shift+V",
    ignoredApps: [],
    locale: "zh-CN",
    density: "compact",
    themeMode: "system",
    accentColor: "amber",
  });
  const showSettings = ref(false);
  const recordingShortcut = ref(false);
  const openSelectKey = ref(null);
  const savingSettings = ref(false);
  const settingsSaveError = ref("");
  const appVersion = ref("");
  const platformCapabilities = ref({
    platform: "windows",
    supportsClipboardWrite: true,
    supportsDirectPaste: true,
    supportsLaunchOnStartup: true,
    supportsMixedReplay: true,
  });

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
  const languageToggleIndex = computed(() =>
    Math.max(localeOptions.findIndex((option) => option.value === settings.locale), 0),
  );
  const debugToggleIndex = computed(() => (settings.debugEnabled ? 0 : 1));
  const launchToggleIndex = computed(() => (settings.launchOnStartup ? 0 : 1));
  const autoCheckUpdatesToggleIndex = computed(() => (settings.autoCheckUpdates ? 0 : 1));
  const canToggleLaunchOnStartup = computed(
    () => platformCapabilities.value.supportsLaunchOnStartup,
  );
  const maxImageBytesMb = computed({
    get: () => Number((settings.maxImageBytes / 1_000_000).toFixed(1)),
    set: (value) => {
      const next = Number(value);
      settings.maxImageBytes = Math.max(
        1_000_000,
        Math.round((Number.isFinite(next) ? next : 1) * 1_000_000),
      );
    },
  });

  function setMaxImageBytesMb(value) {
    maxImageBytesMb.value = value;
  }

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

  function chooseSelectOption(key, field, value) {
    settings[field] = value;
    if (key === "themeMode" || key === "accentColor") {
      closeSelect();
    }
  }

  function formatErrorMessage(error) {
    const code = extractErrorCode(error);
    if (code === "unsupported_launch_on_startup") {
      return t("unsupportedLaunchOnStartup");
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
    return t("saveSettingsFailed");
  }

  function beginShortcutRecording() {
    recordingShortcut.value = true;
  }

  function endShortcutRecording() {
    recordingShortcut.value = false;
  }

  function handleShortcutKeydown(event) {
    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Tab" || event.key === "Escape") {
      endShortcutRecording();
      return;
    }

    if (event.key === "Backspace" || event.key === "Delete") {
      settings.globalShortcut = "";
      endShortcutRecording();
      return;
    }

    const parts = [];
    if (event.ctrlKey) {
      parts.push("Ctrl");
    }
    if (event.altKey) {
      parts.push("Alt");
    }
    if (event.shiftKey) {
      parts.push("Shift");
    }
    if (event.metaKey) {
      parts.push("Meta");
    }

    const mainKey = normalizeShortcutKey(event.key);
    if (!mainKey || ["Ctrl", "Alt", "Shift", "Meta"].includes(mainKey)) {
      return;
    }

    settings.globalShortcut = [...parts, mainKey].join("+");
    endShortcutRecording();
  }

  async function loadAppVersion() {
    appVersion.value = (await getAppVersion().catch(() => "")) || "";
  }

  async function loadPlatformCapabilities() {
    platformCapabilities.value = await fetchPlatformCapabilities().catch(() => ({
      platform: "unknown",
      supportsClipboardWrite: false,
      supportsDirectPaste: false,
      supportsLaunchOnStartup: false,
      supportsMixedReplay: false,
    }));
  }

  async function refreshSettings() {
    const next = await fetchSettings();
    Object.assign(settings, next);
  }

  async function saveSettings(onSaved) {
    if (savingSettings.value) {
      return;
    }

    const payload = { ...settings };
    settingsSaveError.value = "";
    savingSettings.value = true;
    closeSelect();
    showSettings.value = false;

    try {
      await persistSettings(payload);
      Object.assign(settings, payload);
      onSaved?.();
    } catch (error) {
      settingsSaveError.value = formatErrorMessage(error);
      showSettings.value = true;
      console.error("Failed to save settings", error);
    } finally {
      savingSettings.value = false;
    }
  }

  return {
    appVersion,
    autoCheckUpdatesToggleIndex,
    beginShortcutRecording,
    canToggleLaunchOnStartup,
    chooseSelectOption,
    closeSelect,
    currentAccentColor,
    currentAccentColorOptions,
    currentDensity,
    currentLocale,
    currentThemeMode,
    currentThemeModeOptions,
    debugToggleIndex,
    endShortcutRecording,
    handleShortcutKeydown,
    languageToggleIndex,
    launchToggleIndex,
    loadAppVersion,
    loadPlatformCapabilities,
    localeOptions,
    maxImageBytesMb,
    openSelectKey,
    platformCapabilities,
    recordingShortcut,
    refreshSettings,
    saveSettings,
    savingSettings,
    segmentedToggleStyle,
    selectedOptionLabel,
    setMaxImageBytesMb,
    settings,
    settingsSaveError,
    showSettings,
    t,
    toggleSelect,
  };
}
