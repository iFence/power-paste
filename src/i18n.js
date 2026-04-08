export const defaultLocale = "zh-CN";

export const localeOptions = [
  { value: "zh-CN", label: "简体中文" },
  { value: "en-US", label: "English" },
];

export const densityOptions = {
  "zh-CN": [
    { value: "compact", label: "紧凑" },
    { value: "cozy", label: "舒展" },
  ],
  "en-US": [
    { value: "compact", label: "Compact" },
    { value: "cozy", label: "Cozy" },
  ],
};

export const themeModeOptions = {
  "zh-CN": [
    { value: "light", label: "浅色" },
    { value: "dark", label: "深色" },
    { value: "system", label: "系统" },
  ],
  "en-US": [
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
    { value: "system", label: "System" },
  ],
};

export const accentColorOptions = {
  "zh-CN": [
    { value: "ocean", label: "海蓝" },
    { value: "amber", label: "琥珀" },
    { value: "jade", label: "青玉" },
    { value: "rose", label: "玫瑰" },
  ],
  "en-US": [
    { value: "ocean", label: "Ocean" },
    { value: "amber", label: "Amber" },
    { value: "jade", label: "Jade" },
    { value: "rose", label: "Rose" },
  ],
};

export const messages = {
  "zh-CN": {
    appName: "Power Paste",
    navHistory: "历史",
    navSettings: "设置",
    settingsTitle: "设置",
    application: "应用",
    searchPlaceholder: "搜索剪贴板历史",
    clear: "清空历史",
    copy: "复制",
    pin: "置顶",
    unpin: "取消置顶",
    star: "收藏",
    unstar: "取消收藏",
    deleteItem: "删除条目",
    saveChanges: "保存更改",
    launchOnStartup: "开机启动",
    pollingInterval: "轮询间隔 (ms)",
    maxHistoryItems: "最大历史数量",
    maxImageBytes: "图片大小",
    globalShortcut: "全局快捷键",
    ignoredApps: "忽略的应用",
    language: "界面语言",
    density: "列表密度",
    themeMode: "界面主题",
    accentColor: "主题颜色",
    ignoredAppsPlaceholder: "例如 1Password, Bitwarden, KeePassXC",
    loadingHistory: "正在加载历史...",
    historyEmpty: "当前没有剪贴板历史。",
    clipboardFallback: "剪贴板",
    itemCount: "本地已缓存 {count} 条记录，快捷键：{shortcut}",
    localeInstant: "语言切换会立即应用到界面。",
    densityInstant: "可切换更紧凑或更舒展的列表密度。",
    themeModeInstant: "支持浅色、深色和跟随系统。",
    accentColorInstant: "主题颜色会立即应用到当前界面。",
    openLink: "打开链接",
    kindLink: "链接",
    kindText: "文本",
    kindImage: "图片",
    kindMixed: "图文",
    filterMixed: "图文",
    badgePinned: "已置顶",
    badgeStarred: "已收藏",
    statusMonitorOnline: "剪贴板监听已启动",
    statusCopied: "已重新复制到剪贴板",
    statusSaved: "设置已保存",
    statusUpdated: "历史记录已更新",
  },
  "en-US": {
    appName: "Power Paste",
    navHistory: "History",
    navSettings: "Settings",
    settingsTitle: "Settings",
    application: "Application",
    version: "Version",
    searchPlaceholder: "Search clipboard history",
    clear: "Clear",
    copy: "Copy",
    pin: "Pin",
    unpin: "Unpin",
    star: "Star",
    unstar: "Unstar",
    deleteItem: "Delete item",
    editItem: "Edit",
    editTextItem: "Edit Text",
    cancelAction: "Cancel",
    saveChanges: "Save Changes",
    debugMode: "Debug mode",
    toggleOn: "On",
    toggleOff: "Off",
    launchOnStartup: "Launch on startup",
    pollingInterval: "Polling interval (ms)",
    maxHistoryItems: "Max history items",
    maxImageBytes: "Image size",
    megabytesShort: "MB",
    globalShortcut: "Shortcut",
    shortcutPlaceholder: "Focus and press keys",
    shortcutRecording: "Press the shortcut keys",
    ignoredApps: "Ignored apps",
    language: "Language",
    density: "List density",
    themeMode: "Theme",
    accentColor: "Accent color",
    ignoredAppsPlaceholder: "e.g. 1Password, Bitwarden, KeePassXC",
    loadingHistory: "Loading history...",
    historyEmpty: "Clipboard history is empty.",
    clipboardFallback: "Clipboard",
    itemCount: "{count} items cached locally. Shortcut: {shortcut}",
    localeInstant: "Language changes apply to the interface immediately.",
    densityInstant: "Switch between a tighter or more relaxed history list.",
    themeModeInstant: "Choose light, dark, or follow the system theme.",
    accentColorInstant: "Accent color updates the current panel immediately.",
    openLink: "Open Link",
    kindLink: "Link",
    kindText: "Text",
    kindImage: "Image",
    kindMixed: "Image + Text",
    filterAll: "All",
    filterMixed: "Image + Text",
    filterText: "Text",
    filterImage: "Image",
    filterPinned: "Pinned",
    badgePinned: "Pinned",
    badgeStarred: "Starred",
    statusMonitorOnline: "Clipboard monitor online",
    statusCopied: "Copied back to clipboard",
    statusSaved: "Settings saved",
    statusUpdated: "History updated",
  },
};

Object.assign(messages["zh-CN"], {
  version: "版本",
  editItem: "编辑",
  editTextItem: "编辑文本",
  closeAction: "关闭",
  cancelAction: "取消",
  megabytesShort: "MB",
  shortcutPlaceholder: "聚焦后直接按快捷键",
  shortcutRecording: "请按下快捷键组合",
});

Object.assign(messages["zh-CN"], {
  filterAll: "全部",
  filterMixed: "图文",
  filterText: "文本",
  filterImage: "图片",
  filterPinned: "置顶",
});

Object.assign(messages["zh-CN"], {
  debugMode: "璋冭瘯妯″紡",
});

Object.assign(messages["zh-CN"], {
  debugMode: "\u8c03\u8bd5\u6a21\u5f0f",
  toggleOn: "\u5f00\u542f",
  toggleOff: "\u5173\u95ed",
});

localeOptions[0].label = "\u7b80\u4f53\u4e2d\u6587";

densityOptions["zh-CN"] = [
  { value: "compact", label: "\u7d27\u51d1" },
  { value: "cozy", label: "\u8212\u9002" },
];

themeModeOptions["zh-CN"] = [
  { value: "light", label: "\u6d45\u8272" },
  { value: "dark", label: "\u6df1\u8272" },
  { value: "system", label: "\u8ddf\u968f\u7cfb\u7edf" },
];

accentColorOptions["zh-CN"] = [
  { value: "ocean", label: "\u6d77\u84dd" },
  { value: "amber", label: "\u7425\u73c0" },
  { value: "jade", label: "\u9752\u7389" },
  { value: "rose", label: "\u73ab\u7470" },
];

Object.assign(messages["zh-CN"], {
  unsupportedCurrentPlatform: "\u5f53\u524d\u5e73\u53f0\u6682\u4e0d\u652f\u6301\u8be5\u64cd\u4f5c",
  unsupportedClipboardWrite: "\u5f53\u524d\u5e73\u53f0\u6682\u4e0d\u652f\u6301\u5199\u5165\u7cfb\u7edf\u526a\u8d34\u677f",
  unsupportedDirectPaste: "\u5f53\u524d\u5e73\u53f0\u6682\u4e0d\u652f\u6301\u76f4\u63a5\u7c98\u8d34\u5230\u76ee\u6807\u5e94\u7528",
  unsupportedLaunchOnStartup: "\u5f53\u524d\u5e73\u53f0\u6682\u4e0d\u652f\u6301\u5f00\u673a\u542f\u52a8",
  checkForUpdates: "\u68c0\u67e5\u66f4\u65b0",
  downloadAndInstall: "\u4e0b\u8f7d\u5e76\u5b89\u88c5",
  updateIdle: "\u5c1a\u672a\u6267\u884c\u66f4\u65b0\u68c0\u67e5",
  checkingForUpdates: "\u6b63\u5728\u68c0\u67e5\u66f4\u65b0",
  updateAvailable: "\u53d1\u73b0\u65b0\u7248\u672c",
  updateAvailableVersion: "\u53d1\u73b0\u65b0\u7248\u672c {version}",
  updateConfirmInstall: "\u53d1\u73b0\u65b0\u7248\u672c\uff0c\u662f\u5426\u7acb\u5373\u4e0b\u8f7d\u5e76\u5b89\u88c5\uff1f",
  updateConfirmInstallVersion: "\u53d1\u73b0\u65b0\u7248\u672c {version}\uff0c\u662f\u5426\u7acb\u5373\u4e0b\u8f7d\u5e76\u5b89\u88c5\uff1f",
  downloadingUpdate: "\u6b63\u5728\u4e0b\u8f7d\u66f4\u65b0",
  downloadingUpdateProgress: "\u6b63\u5728\u4e0b\u8f7d\u66f4\u65b0 {percent}%",
  updateReadyToInstall: "\u66f4\u65b0\u5df2\u4e0b\u8f7d\uff0c\u5b89\u88c5\u7a0b\u5e8f\u5373\u5c06\u542f\u52a8",
  upToDate: "\u5f53\u524d\u5df2\u662f\u6700\u65b0\u7248\u672c",
  updateCheckFailed: "\u68c0\u67e5\u66f4\u65b0\u5931\u8d25",
  updateInstallFailed: "\u5b89\u88c5\u66f4\u65b0\u5931\u8d25",
  currentVersionLabel: "\u5f53\u524d\u7248\u672c\uff1a{version}",
  latestVersionLabel: "\u6700\u65b0\u7248\u672c\uff1a{version}",
  saveSettingsFailed: "\u4fdd\u5b58\u8bbe\u7f6e\u5931\u8d25",
});

Object.assign(messages["en-US"], {
  unsupportedCurrentPlatform: "This action is not available on the current platform.",
  unsupportedClipboardWrite: "Writing back to the system clipboard is not available on this platform.",
  unsupportedDirectPaste: "Direct paste into the target app is not available on this platform.",
  unsupportedLaunchOnStartup: "Launch on startup is not available on this platform.",
  checkForUpdates: "Check for Updates",
  downloadAndInstall: "Download and Install",
  updateIdle: "No update check has been run yet.",
  checkingForUpdates: "Checking for updates...",
  updateAvailable: "An update is available.",
  updateAvailableVersion: "Version {version} is available.",
  updateConfirmInstall: "An update is available. Download and install it now?",
  updateConfirmInstallVersion: "Version {version} is available. Download and install it now?",
  downloadingUpdate: "Downloading update...",
  downloadingUpdateProgress: "Downloading update... {percent}%",
  updateReadyToInstall: "The update is ready and the installer will start shortly.",
  upToDate: "You're on the latest version.",
  updateCheckFailed: "Failed to check for updates.",
  updateInstallFailed: "Failed to install the update.",
  currentVersionLabel: "Current version: {version}",
  latestVersionLabel: "Latest version: {version}",
  saveSettingsFailed: "Failed to save settings",
});

export function translate(locale, key, params = {}) {
  const pack = messages[locale] || messages[defaultLocale];
  const fallback = messages["en-US"];
  const template = pack[key] ?? fallback[key] ?? key;
  return template.replace(/\{(\w+)\}/g, (_, name) => `${params[name] ?? ""}`);
}
