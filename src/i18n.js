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
    settingsCategoryGeneral: "通用",
    settingsCategoryHistory: "历史",
    settingsCategoryTransfer: "互传",
    settingsCategoryShortcuts: "快捷键",
    settingsCategoryAdvanced: "高级",
    settingsCategoryAbout: "关于",
    resetSettings: "重置设置",
    resetSettingsConfirm: "确定要恢复默认设置吗？窗口位置和大小不会被重置。",
    resetSettingsTip: "恢复设置页可见配置，窗口位置和大小不会被重置。",
    application: "应用",
    searchPlaceholder: "搜索剪贴板历史",
    clearSearch: "清空搜索",
    clear: "清空历史",
    clearHistoryConfirm: "确定要清空未置顶的历史记录吗？已置顶内容会保留。",
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
    maxHistoryDays: "最大保留天数",
    maxImageBytes: "图片大小",
    copySound: "复制音效",
    lanTransferDownloadDir: "互传文件保存位置",
    lanTransferDownloadDirPlaceholder: "请选择文件保存位置",
    chooseFolder: "选择文件夹",
    globalShortcut: "全局快捷键",
    ignoredApps: "忽略的应用",
    language: "界面语言",
    density: "列表密度",
    themeMode: "界面主题",
    accentColor: "主题颜色",
    aboutTitle: "关于",
    githubRepoLabel: "GitHub 仓库",
    ignoredAppsPlaceholder: "例如 1Password, Bitwarden, KeePassXC",
    launchOnStartupTip: "应用启动时自动运行 Power Paste。",
    maxHistoryItemsTip: "超过数量上限时，会优先清理未置顶的旧记录。",
    maxHistoryDaysTip: "超过保留天数的未置顶历史记录会被自动清理。",
    maxImageBytesTip: "超过该大小的图片不会写入历史记录。",
    lanTransferDownloadDirTip: "手机电脑互传接收到的文件会保存到这个目录。",
    globalShortcutTip: "按下这个全局快捷键可唤起或隐藏主窗口。",
    debugModeTip: "开启后允许开发者工具和调试快捷键。",
    loadingHistory: "正在加载历史...",
    historyEmpty: "当前没有剪贴板历史。",
    startupLoadFailed: "应用初始化失败",
    retryAction: "重试",
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
    historyTags: "标签",
    manageTags: "管理标签",
    removeTag: "移除标签",
    tagNames: "标签名称",
    tagNamesTip: "颜色固定不变，可为每种标签颜色设置自己的显示名称。",
    tagDefaultNameRed: "红色",
    tagDefaultNameOrange: "橙色",
    tagDefaultNameYellow: "黄色",
    tagDefaultNameGreen: "绿色",
    tagDefaultNameBlue: "蓝色",
    tagDefaultNamePurple: "紫色",
    tagDefaultNameGray: "灰色",
    tagColorRed: "红色标签",
    tagColorOrange: "橙色标签",
    tagColorYellow: "黄色标签",
    tagColorGreen: "绿色标签",
    tagColorBlue: "蓝色标签",
    tagColorPurple: "紫色标签",
    tagColorGray: "灰色标签",
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
    settingsCategoryGeneral: "General",
    settingsCategoryHistory: "History",
    settingsCategoryTransfer: "Transfer",
    settingsCategoryShortcuts: "Shortcuts",
    settingsCategoryAdvanced: "Advanced",
    settingsCategoryAbout: "About",
    resetSettings: "Reset Settings",
    resetSettingsConfirm:
      "Restore default settings? Window position and size will not be reset.",
    resetSettingsTip:
      "Restore visible settings. Window position and size will not be reset.",
    application: "Application",
    version: "Version",
    searchPlaceholder: "Search clipboard history",
    clearSearch: "Clear search",
    clear: "Clear",
    clearHistoryConfirm: "Clear all unpinned history items? Pinned items will be kept.",
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
    maxHistoryDays: "Max retention days",
    maxImageBytes: "Image size",
    copySound: "Copy sound",
    lanTransferDownloadDir: "Transfer download folder",
    lanTransferDownloadDirPlaceholder: "Choose a download folder",
    chooseFolder: "Choose folder",
    megabytesShort: "MB",
    globalShortcut: "Shortcut",
    shortcutPlaceholder: "Focus and press keys",
    shortcutRecording: "Press the shortcut keys",
    ignoredApps: "Ignored apps",
    language: "Language",
    density: "List density",
    themeMode: "Theme",
    accentColor: "Accent color",
    aboutTitle: "About",
    githubRepoLabel: "GitHub Repository",
    ignoredAppsPlaceholder: "e.g. 1Password, Bitwarden, KeePassXC",
    launchOnStartupTip: "Run Power Paste automatically when the system starts.",
    maxHistoryItemsTip: "When the limit is exceeded, old unpinned items are removed first.",
    maxHistoryDaysTip: "Unpinned history older than this many days is removed automatically.",
    maxImageBytesTip: "Images larger than this limit are not stored in history.",
    lanTransferDownloadDirTip: "Files received from phone and PC transfer are saved here.",
    globalShortcutTip: "Use this global shortcut to show or hide the main window.",
    debugModeTip: "Allows developer tools and debugging keyboard shortcuts.",
    loadingHistory: "Loading history...",
    historyEmpty: "Clipboard history is empty.",
    startupLoadFailed: "Failed to initialize the app.",
    retryAction: "Retry",
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
    historyTags: "Tags",
    manageTags: "Manage Tags",
    removeTag: "Remove tag",
    tagNames: "Tag Names",
    tagNamesTip: "Colors stay fixed. You can customize the display name for each color.",
    tagDefaultNameRed: "Red",
    tagDefaultNameOrange: "Orange",
    tagDefaultNameYellow: "Yellow",
    tagDefaultNameGreen: "Green",
    tagDefaultNameBlue: "Blue",
    tagDefaultNamePurple: "Purple",
    tagDefaultNameGray: "Gray",
    tagColorRed: "Red tag",
    tagColorOrange: "Orange tag",
    tagColorYellow: "Yellow tag",
    tagColorGreen: "Green tag",
    tagColorBlue: "Blue tag",
    tagColorPurple: "Purple tag",
    tagColorGray: "Gray tag",
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
  unsupportedCurrentPlatform:
    "\u5f53\u524d\u5e73\u53f0\u6682\u4e0d\u652f\u6301\u8be5\u64cd\u4f5c",
  unsupportedClipboardWrite:
    "\u5f53\u524d\u5e73\u53f0\u6682\u4e0d\u652f\u6301\u5199\u5165\u7cfb\u7edf\u526a\u8d34\u677f",
  unsupportedDirectPaste:
    "\u5f53\u524d\u5e73\u53f0\u6682\u4e0d\u652f\u6301\u76f4\u63a5\u7c98\u8d34\u5230\u76ee\u6807\u5e94\u7528",
  linuxX11ToolsMissing:
    "Linux \u76f4\u63a5\u7c98\u8d34\u4f9d\u8d56 xdotool\uff0c\u8bf7\u5148\u5b89\u88c5 xdotool \u540e\u91cd\u8bd5",
  linuxWaylandToolsMissing:
    "Wayland \u4f1a\u8bdd\u4e0b\u7684 Linux \u76f4\u63a5\u7c98\u8d34\u4f9d\u8d56 wtype\uff0c\u8bf7\u5148\u5b89\u88c5 wtype \u540e\u91cd\u8bd5",
  pasteTargetFocusFailed:
    "\u672a\u80fd\u6062\u590d\u76ee\u6807\u7a97\u53e3\u7126\u70b9\uff0c\u5df2\u53d6\u6d88\u672c\u6b21\u7c98\u8d34",
  pasteTargetPermissionDenied:
    "\u7cfb\u7edf\u672a\u6388\u4e88 Power Paste \u8f85\u52a9\u529f\u80fd\u6216\u81ea\u52a8\u5316\u6743\u9650\uff0c\u8bf7\u5728\u201c\u7cfb\u7edf\u8bbe\u7f6e > \u9690\u79c1\u4e0e\u5b89\u5168\u6027 > \u8f85\u52a9\u529f\u80fd / \u81ea\u52a8\u5316\u201d\u4e2d\u5141\u8bb8\u540e\u91cd\u8bd5",
  unsupportedLaunchOnStartup:
    "\u5f53\u524d\u5e73\u53f0\u6682\u4e0d\u652f\u6301\u5f00\u673a\u542f\u52a8",
  checkForUpdates: "\u68c0\u67e5\u66f4\u65b0",
  downloadAndInstall: "\u4e0b\u8f7d\u5e76\u5b89\u88c5",
  updateIdle: "\u5c1a\u672a\u6267\u884c\u66f4\u65b0\u68c0\u67e5",
  checkingForUpdates: "\u6b63\u5728\u68c0\u67e5\u66f4\u65b0",
  updateAvailable: "\u53d1\u73b0\u65b0\u7248\u672c",
  updateAvailableVersion: "\u53d1\u73b0\u65b0\u7248\u672c {version}",
  updateConfirmInstall:
    "\u53d1\u73b0\u65b0\u7248\u672c\uff0c\u662f\u5426\u7acb\u5373\u4e0b\u8f7d\u5e76\u5b89\u88c5\uff1f",
  updateConfirmInstallVersion:
    "\u53d1\u73b0\u65b0\u7248\u672c {version}\uff0c\u662f\u5426\u7acb\u5373\u4e0b\u8f7d\u5e76\u5b89\u88c5\uff1f",
  downloadingUpdate: "\u6b63\u5728\u4e0b\u8f7d\u66f4\u65b0",
  downloadingUpdateProgress: "\u6b63\u5728\u4e0b\u8f7d\u66f4\u65b0 {percent}%",
  updateReadyToInstall:
    "\u66f4\u65b0\u5df2\u4e0b\u8f7d\uff0c\u5b89\u88c5\u7a0b\u5e8f\u5373\u5c06\u542f\u52a8",
  upToDate: "\u5f53\u524d\u5df2\u662f\u6700\u65b0\u7248\u672c",
  updateCheckFailed: "\u68c0\u67e5\u66f4\u65b0\u5931\u8d25",
  updateInstallFailed: "\u5b89\u88c5\u66f4\u65b0\u5931\u8d25",
  currentVersionLabel: "\u5f53\u524d\u7248\u672c\uff1a{version}",
  latestVersionLabel: "\u6700\u65b0\u7248\u672c\uff1a{version}",
  updateDetailsTitle: "\u66f4\u65b0\u5185\u5bb9",
  updateNotesEmpty: "\u6682\u65e0\u66f4\u65b0\u8bf4\u660e",
  ignoreUpdate: "\u5ffd\u7565",
  installUpdateNow: "\u5b89\u88c5",
  updateDebugTitle: "\u66f4\u65b0\u8c03\u8bd5",
  updateDebugHint:
    "\u4ec5\u5f00\u53d1\u6a21\u5f0f\u53ef\u89c1\uff0c\u7528\u4e8e\u9884\u89c8 new \u5fbd\u6807\u548c\u66f4\u65b0\u72b6\u6001\u3002",
  updateDebugVersionLabel: "\u8c03\u8bd5\u7248\u672c\u53f7",
  updateDebugVersionPlaceholder: "\u4f8b\u5982 v0.3.6",
  updateDebugBodyLabel: "\u8c03\u8bd5\u66f4\u65b0\u8bf4\u660e",
  updateDebugBodyPlaceholder:
    "\u5728\u8fd9\u91cc\u8f93\u5165 Markdown \u66f4\u65b0\u8bf4\u660e",
  updateDebugAvailable: "\u65b0\u7248\u672c",
  updateDebugDownloading: "\u4e0b\u8f7d\u4e2d",
  updateDebugDownloaded: "\u5df2\u4e0b\u8f7d",
  updateDebugUpToDate: "\u5df2\u662f\u6700\u65b0",
  updateDebugError: "\u9519\u8bef",
  updateDebugClear: "\u6062\u590d\u771f\u5b9e\u68c0\u67e5",
  saveSettingsFailed: "\u4fdd\u5b58\u8bbe\u7f6e\u5931\u8d25",
  backAction: "\u8fd4\u56de",
  lanReceiverTitle: "\u624b\u673a\u7535\u8111\u4e92\u4f20",
  lanReceiverSubtitle:
    "\u626b\u7801\u540e\u7528\u624b\u673a\u6d4f\u89c8\u5668\u4e0e\u7535\u8111\u4e92\u4f20\u6587\u5b57\u3001\u56fe\u7247\u548c\u6587\u4ef6",
  lanReceiverStatus: "\u8fde\u63a5\u72b6\u6001",
  lanReceiverReady: "\u7b49\u5f85\u4e92\u4f20\u6d88\u606f",
  lanReceiverStopped: "\u5df2\u505c\u6b62",
  lanReceiverReceivedText:
    "\u5df2\u63a5\u6536\u6587\u672c\u5e76\u5199\u5165\u526a\u8d34\u677f",
  lanReceiverReceivedImage:
    "\u5df2\u63a5\u6536\u56fe\u7247\u5e76\u5199\u5165\u526a\u8d34\u677f",
  lanReceiverReceivedFile: "\u5df2\u63a5\u6536\u6587\u4ef6\u5e76\u4fdd\u5b58",
  lanReceiverProcessingImage:
    "\u56fe\u7247\u5df2\u4e0a\u4f20\uff0c\u6b63\u5728\u5199\u5165\u684c\u9762\u526a\u8d34\u677f",
  lanReceiverFailed: "\u63a5\u6536\u5931\u8d25",
  lanTransferTitle: "\u624b\u673a\u7535\u8111\u4e92\u4f20",
  lanTransferPhone: "\u624b\u673a",
  lanTransferDesktop: "\u7535\u8111",
  lanTransferPort: "\u7aef\u53e3",
  lanTransferConnected: "\u5df2\u8fde\u63a5",
  lanTransferDisconnected: "\u5df2\u65ad\u5f00",
  lanTransferConnectedDevices: "\u8fde\u63a5\u8bbe\u5907\u6570",
  lanTransferEmpty:
    "\u626b\u7801\u540e\u5f00\u59cb\u5bf9\u8bdd\uff0c\u6587\u5b57\u4f1a\u8fdb\u5165\u526a\u8d34\u677f\uff0c\u6587\u4ef6\u4f1a\u4fdd\u5b58\u5230\u4e0b\u8f7d\u76ee\u5f55\u3002",
  lanTransferFile: "\u6587\u4ef6",
  lanTransferChooseFile: "\u9009\u62e9\u56fe\u7247\u6216\u6587\u4ef6",
  lanTransferInputPlaceholder:
    "\u8f93\u5165\u8981\u53d1\u9001\u5230\u624b\u673a\u7684\u6587\u5b57",
  lanTransferSend: "\u53d1\u9001",
  lanTransferTooManyFiles:
    "\u4e00\u6b21\u6700\u591a\u9009\u62e9 {max} \u4e2a\u6587\u4ef6\u6216\u56fe\u7247\uff0c\u5df2\u53ea\u53d1\u9001\u524d {max} \u4e2a",
  lanTransferUploading: "\u4e0a\u4f20\u4e2d {progress}%",
  lanTransferUploadFailed: "\u4e0a\u4f20\u5931\u8d25",
  openAction: "\u6253\u5f00",
  revealInExplorer:
    "\u5728\u6587\u4ef6\u8d44\u6e90\u7ba1\u7406\u5668\u6253\u5f00",
  lanTransferDownloadDirMissing:
    "\u4e92\u4f20\u6587\u4ef6\u4fdd\u5b58\u76ee\u5f55\u4e0d\u5b58\u5728",
  lanTransferDownloadDirNotDirectory:
    "\u4e92\u4f20\u6587\u4ef6\u4fdd\u5b58\u4f4d\u7f6e\u4e0d\u662f\u76ee\u5f55",
  lanTransferDownloadDirNotWritable:
    "\u4e92\u4f20\u6587\u4ef6\u4fdd\u5b58\u76ee\u5f55\u4e0d\u53ef\u5199",
});

Object.assign(messages["en-US"], {
  unsupportedCurrentPlatform:
    "This action is not available on the current platform.",
  unsupportedClipboardWrite:
    "Writing back to the system clipboard is not available on this platform.",
  unsupportedDirectPaste:
    "Direct paste into the target app is not available on this platform.",
  linuxX11ToolsMissing:
    "Direct paste on Linux requires xdotool. Install xdotool and try again.",
  linuxWaylandToolsMissing:
    "Direct paste on Wayland requires wtype. Install wtype and try again.",
  pasteTargetFocusFailed:
    "The target window could not be focused. Paste was cancelled.",
  pasteTargetPermissionDenied:
    "Power Paste does not have the required Accessibility or Automation permission. Allow it in System Settings > Privacy & Security > Accessibility / Automation and try again.",
  unsupportedLaunchOnStartup:
    "Launch on startup is not available on this platform.",
  checkForUpdates: "Check for Updates",
  downloadAndInstall: "Download and Install",
  updateIdle: "No update check has been run yet.",
  checkingForUpdates: "Checking for updates...",
  updateAvailable: "An update is available.",
  updateAvailableVersion: "Version {version} is available.",
  updateConfirmInstall: "An update is available. Download and install it now?",
  updateConfirmInstallVersion:
    "Version {version} is available. Download and install it now?",
  downloadingUpdate: "Downloading update...",
  downloadingUpdateProgress: "Downloading update... {percent}%",
  updateReadyToInstall:
    "The update is ready and the installer will start shortly.",
  upToDate: "You're on the latest version.",
  updateCheckFailed: "Failed to check for updates.",
  updateInstallFailed: "Failed to install the update.",
  currentVersionLabel: "Current version: {version}",
  latestVersionLabel: "Latest version: {version}",
  updateDetailsTitle: "What's New",
  updateNotesEmpty: "No release notes were provided for this version.",
  ignoreUpdate: "Ignore",
  installUpdateNow: "Install",
  updateDebugTitle: "Update Debug",
  updateDebugHint:
    "Visible in development only to preview the new badge and update states.",
  updateDebugVersionLabel: "Debug Version",
  updateDebugVersionPlaceholder: "For example v0.3.6",
  updateDebugBodyLabel: "Debug Notes",
  updateDebugBodyPlaceholder: "Enter Markdown release notes here",
  updateDebugAvailable: "Available",
  updateDebugDownloading: "Downloading",
  updateDebugDownloaded: "Downloaded",
  updateDebugUpToDate: "Up to Date",
  updateDebugError: "Error",
  updateDebugClear: "Use Real Check",
  saveSettingsFailed: "Failed to save settings",
  backAction: "Back",
  lanReceiverTitle: "Phone and PC Transfer",
  lanReceiverSubtitle:
    "Scan with a phone browser to exchange text, images, and files.",
  lanReceiverStatus: "Status",
  lanReceiverReady: "Waiting for transfer messages",
  lanReceiverStopped: "Stopped",
  lanReceiverReceivedText: "Text received and copied to the clipboard",
  lanReceiverReceivedImage: "Image received and copied to the clipboard",
  lanReceiverReceivedFile: "File received and saved",
  lanReceiverProcessingImage:
    "Image uploaded and is being copied to the desktop clipboard",
  lanReceiverFailed: "Receive failed",
  lanTransferTitle: "Phone and PC Transfer",
  lanTransferPhone: "Phone",
  lanTransferDesktop: "Desktop",
  lanTransferPort: "Port",
  lanTransferConnected: "Connected",
  lanTransferDisconnected: "Disconnected",
  lanTransferConnectedDevices: "Connected devices",
  lanTransferEmpty:
    "Scan the QR code and start chatting. Text goes to the clipboard; files are saved to the download folder.",
  lanTransferFile: "File",
  lanTransferChooseFile: "Choose image or file",
  lanTransferInputPlaceholder: "Type text to send to the phone",
  lanTransferSend: "Send",
  lanTransferTooManyFiles:
    "You can choose up to {max} files or images at once. Only the first {max} will be sent.",
  lanTransferUploading: "Uploading {progress}%",
  lanTransferUploadFailed: "Upload failed",
  openAction: "Open",
  revealInExplorer: "Show in File Explorer",
  lanTransferDownloadDirMissing: "The transfer download folder does not exist.",
  lanTransferDownloadDirNotDirectory:
    "The transfer download location is not a folder.",
  lanTransferDownloadDirNotWritable:
    "The transfer download folder is not writable.",
});

export function translate(locale, key, params = {}) {
  const pack = messages[locale] || messages[defaultLocale];
  const fallback = messages["en-US"];
  const template = pack[key] ?? fallback[key] ?? key;
  return template.replace(/\{(\w+)\}/g, (_, name) => `${params[name] ?? ""}`);
}
