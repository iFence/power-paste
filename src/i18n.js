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
    copyStatsEnabled: "复制次数统计",
    copyStatsEnabledTip:
      "开启后，重新复制历史记录会累计次数，并优先显示复制次数更多的内容。",
    copyCountLabel: "复制 {count} 次",
    pasteStatsEnabled: "粘贴次数统计",
    pasteStatsEnabledTip:
      "开启后，通过 Power Paste 直接粘贴历史记录会累计次数，并优先显示粘贴次数更多的内容。",
    pasteCountLabel: "粘贴 {count} 次",
    copySound: "复制音效",
    lanTransferDownloadDir: "互传文件保存位置",
    lanTransferDownloadDirPlaceholder: "请选择文件保存位置",
    chooseFolder: "选择文件夹",
    globalShortcut: "全局快捷键",
    quickPasteShortcut: "快速粘贴快捷键",
    searchShortcut: "搜索快捷键",
    filterShortcut: "筛选切换快捷键",
    ignoredApps: "忽略的应用",
    language: "界面语言",
    density: "列表密度",
    themeMode: "界面主题",
    accentColor: "主题颜色",
    aboutTitle: "关于",
    githubRepoLabel: "GitHub 仓库",
    landingPageLabel: "访问官网",
    recommendedAppsLabel: "推荐应用",
    stewardTooltip:
      "Steward 是采用 Rust、GPUI 与 gpui-component 构建的一款小巧精致的 Windows 启动器与插件平台。",
    lumiaTooltip:
      "Lumia 是 power-paste 作者采用纯 Rust 开发的一款小巧、精致、高颜值、高性能的跨平台图片查看器。",
    ignoredAppsPlaceholder: "例如 1Password, Bitwarden, KeePassXC",
    launchOnStartupTip: "应用启动时自动运行 Power Paste。",
    maxHistoryItemsTip: "超过数量上限时，会优先清理未置顶的旧记录。",
    maxHistoryDaysTip: "超过保留天数的未置顶历史记录会被自动清理。",
    maxImageBytesTip: "超过该大小的图片不会写入历史记录。",
    lanTransferDownloadDirTip: "局域网互传接收到的文件会保存到这个目录。",
    lanTransferEnabled: "启用局域网互传",
    lanTransferEnabledTip:
      "开启后本机会监听 LocalSend 默认端口 53317，并在局域网内被官方 LocalSend 客户端发现。",
    lanDeviceAlias: "设备名称",
    lanDeviceAliasTip: "其他设备上显示的名字，留空则使用主机名。",
    lanDeviceAliasPlaceholder: "留空使用主机名",
    lanReceivePolicy: "接收策略",
    lanReceivePolicyTip:
      "每次询问：未信任的设备需要确认；全部自动接受：任何设备都不再询问。已信任设备始终直接接受。",
    lanReceivePolicyAsk: "每次询问",
    lanReceivePolicyAuto: "全部自动接受",
    lanTrustedDevices: "已信任设备",
    lanTrustedDevicesTip: "来自这些设备的传输请求会直接接受，不再弹出确认。",
    lanTrustedUnknownDevice: "未知设备",
    globalShortcutTip: "按下这个全局快捷键可唤起或隐藏主窗口。",
    quickPasteShortcutTip:
      "按住快捷键唤起主窗口；不松开 Ctrl 时重复按 ` 向下选择，松开 Ctrl 后粘贴当前条目。",
    searchShortcutTip: "在主面板内按下该快捷键可聚焦搜索框。默认 Ctrl/Cmd+F。",
    filterShortcutTip:
      "在主面板内按下该快捷键可切换全部、置顶、文本、图片和图文筛选；加 Shift 反向切换。默认 Ctrl+Tab。",
    hardwareAcceleration: "硬件加速",
    hardwareAccelerationTip:
      "关闭后会让 WebView2 以禁用 GPU 加速参数启动，降低内存占用的同时可能影响渲染性能。重启应用后生效。",
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
    clearHistoryConfirm:
      "Clear all unpinned history items? Pinned items will be kept.",
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
    copyStatsEnabled: "Copy count stats",
    copyStatsEnabledTip:
      "When enabled, copying history items again counts usage and moves frequently copied items higher.",
    copyCountLabel: "Copied {count} times",
    pasteStatsEnabled: "Paste count stats",
    pasteStatsEnabledTip:
      "When enabled, direct pasting history items counts usage and moves frequently pasted items higher.",
    pasteCountLabel: "Pasted {count} times",
    copySound: "Copy sound",
    lanTransferDownloadDir: "Transfer download folder",
    lanTransferDownloadDirPlaceholder: "Choose a download folder",
    chooseFolder: "Choose folder",
    megabytesShort: "MB",
    globalShortcut: "Shortcut",
    quickPasteShortcut: "Quick paste shortcut",
    searchShortcut: "Search shortcut",
    filterShortcut: "Filter shortcut",
    shortcutPlaceholder: "Focus and press keys",
    shortcutRecording: "Press the shortcut keys",
    ignoredApps: "Ignored apps",
    language: "Language",
    density: "List density",
    themeMode: "Theme",
    accentColor: "Accent color",
    aboutTitle: "About",
    githubRepoLabel: "GitHub Repository",
    landingPageLabel: "Website",
    recommendedAppsLabel: "Recommended Apps",
    stewardTooltip:
      "Steward is a small, polished launcher and plugin platform for Windows, built with Rust, GPUI, and gpui-component.",
    lumiaTooltip:
      "Lumia is a compact, refined, gorgeous, high-performance cross-platform image viewer developed in pure Rust by the author of Power Paste.",
    ignoredAppsPlaceholder: "e.g. 1Password, Bitwarden, KeePassXC",
    launchOnStartupTip: "Run Power Paste automatically when the system starts.",
    maxHistoryItemsTip:
      "When the limit is exceeded, old unpinned items are removed first.",
    maxHistoryDaysTip:
      "Unpinned history older than this many days is removed automatically.",
    maxImageBytesTip:
      "Images larger than this limit are not stored in history.",
    lanTransferDownloadDirTip:
      "Files received over the LAN are saved to this folder.",
    lanTransferEnabled: "Enable LAN transfer",
    lanTransferEnabledTip:
      "When on, this device listens on the LocalSend default port 53317 and is discoverable by official LocalSend clients on the LAN.",
    lanDeviceAlias: "Device name",
    lanDeviceAliasTip:
      "The name other devices see. Leave empty to use the host name.",
    lanDeviceAliasPlaceholder: "Leave empty to use the host name",
    lanReceivePolicy: "Receive policy",
    lanReceivePolicyTip:
      "Ask every time: untrusted devices need confirmation. Accept automatically: no prompt for any device. Trusted devices are always accepted.",
    lanReceivePolicyAsk: "Ask every time",
    lanReceivePolicyAuto: "Accept automatically (all devices)",
    lanTrustedDevices: "Trusted devices",
    lanTrustedDevicesTip:
      "Transfers from these devices are accepted without a confirmation prompt.",
    lanTrustedUnknownDevice: "Unknown device",
    globalShortcutTip:
      "Use this global shortcut to show or hide the main window.",
    quickPasteShortcutTip:
      "Hold the shortcut to open the main window. While Ctrl stays pressed, press ` again to move down, then release Ctrl to paste the selected item.",
    searchShortcutTip:
      "Focuses the search field inside the main panel. Defaults to Ctrl/Cmd+F.",
    filterShortcutTip:
      "Cycles All, Pinned, Text, Image, and Image + Text filters inside the main panel. Add Shift to cycle backward. Defaults to Ctrl+Tab.",
    hardwareAcceleration: "Hardware acceleration",
    hardwareAccelerationTip:
      "When off, WebView2 starts with GPU acceleration disabled. This may reduce memory usage, but can affect rendering performance. Takes effect after restarting the app.",
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
    filterToday: "Today",
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
    tagNamesTip:
      "Colors stay fixed. You can customize the display name for each color.",
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
  filterToday: "今天",
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
  hardwareAcceleration: "\u786c\u4ef6\u52a0\u901f",
  hardwareAccelerationTip:
    "\u5173\u95ed\u540e\u4f1a\u8ba9 WebView2 \u4ee5\u7981\u7528 GPU \u52a0\u901f\u53c2\u6570\u542f\u52a8\uff0c\u53ef\u80fd\u964d\u4f4e\u5185\u5b58\u5360\u7528\uff0c\u4e5f\u53ef\u80fd\u5f71\u54cd\u6e32\u67d3\u6027\u80fd\u3002\u91cd\u542f\u5e94\u7528\u540e\u751f\u6548\u3002",
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
    "Linux\uff08X11\uff09\u76f4\u63a5\u7c98\u8d34\u9700\u8981\u989d\u5916\u5b89\u88c5 xdotool\u3002\u53ef\u53c2\u8003\uff1aUbuntu/Debian \u6267\u884c `sudo apt install xdotool`\uff0cFedora \u6267\u884c `sudo dnf install xdotool`\uff0cArch \u6267\u884c `sudo pacman -S xdotool`\uff0c\u5b89\u88c5\u540e\u91cd\u8bd5\u5373\u53ef\u3002",
  linuxWaylandToolsMissing:
    "Linux\uff08Wayland\uff09\u76f4\u63a5\u7c98\u8d34\u9700\u8981\u989d\u5916\u5b89\u88c5 wtype\u3002\u53ef\u53c2\u8003\uff1aUbuntu/Debian \u6267\u884c `sudo apt install wtype`\uff0cFedora \u6267\u884c `sudo dnf install wtype`\uff0cArch \u6267\u884c `sudo pacman -S wtype`\uff0c\u5b89\u88c5\u540e\u91cd\u8bd5\u5373\u53ef\u3002",
  pasteTargetFocusFailed:
    "\u672a\u80fd\u6062\u590d\u76ee\u6807\u7a97\u53e3\u7126\u70b9\uff0c\u5df2\u53d6\u6d88\u672c\u6b21\u7c98\u8d34",
  pasteTargetPermissionDenied:
    "\u7cfb\u7edf\u672a\u6388\u4e88 Power Paste \u8f85\u52a9\u529f\u80fd\u6216\u81ea\u52a8\u5316\u6743\u9650\uff0c\u8bf7\u5728\u201c\u7cfb\u7edf\u8bbe\u7f6e > \u9690\u79c1\u4e0e\u5b89\u5168\u6027 > \u8f85\u52a9\u529f\u80fd / \u81ea\u52a8\u5316\u201d\u4e2d\u5141\u8bb8\u540e\u91cd\u8bd5",
  unsupportedLaunchOnStartup:
    "\u5f53\u524d\u5e73\u53f0\u6682\u4e0d\u652f\u6301\u5f00\u673a\u542f\u52a8",
  duplicateShortcut: "\u5feb\u6377\u952e\u4e0d\u80fd\u91cd\u590d",
  invalidShortcut: "\u5feb\u6377\u952e\u683c\u5f0f\u4e0d\u6b63\u786e",
  shortcutRegistrationFailed:
    "\u5feb\u6377\u952e\u672a\u751f\u6548\uff0c\u53ef\u80fd\u5df2\u88ab\u5176\u4ed6\u7a0b\u5e8f\u5360\u7528",
  shortcutConflictMessage:
    "{name}\uff08{shortcut}\uff09\u672a\u751f\u6548\uff0c\u53ef\u80fd\u5df2\u88ab\u5176\u4ed6\u7a0b\u5e8f\u5360\u7528\u3002\u8bf7\u5173\u95ed\u5360\u7528\u7a0b\u5e8f\u540e\u91cd\u8bd5\uff0c\u6216\u5728\u8bbe\u7f6e\u4e2d\u66f4\u6362\u5feb\u6377\u952e\u3002",
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
  lanReceiverTitle: "LocalSend",
  lanTransferStatusRunning: "\u5df2\u5f00\u542f",
  lanTransferStatusStopped: "\u5df2\u505c\u6b62",
  lanTransferStatusPortInUse:
    "\u7aef\u53e3 53317 \u88ab\u5360\u7528\uff0c\u53ef\u80fd\u672c\u673a LocalSend \u6b63\u5728\u8fd0\u884c",
  lanTransferStatusActive: "\u4f20\u8f93\u4e2d",
  lanTransferStatusDone: "\u5df2\u5b8c\u6210",
  lanTransferStatusFailed: "\u5931\u8d25",
  lanTransferStatusCancelled: "\u5df2\u53d6\u6d88",
  lanTransferDevices: "\u9644\u8fd1\u8bbe\u5907",
  lanTransferRefresh: "\u5237\u65b0",
  lanScanMenuTitle: "\u9009\u62e9\u7f51\u6bb5",
  lanScanMenuHint:
    "\u53ea\u626b\u63cf\u6240\u9009\u7f51\u6bb5\uff0c\u5176\u4f59\u7f51\u6bb5\u4e0d\u4f1a\u88ab\u63a2\u6d4b\u3002",
  lanScanLastUsed: "\u4e0a\u6b21",
  lanScanCancel: "\u53d6\u6d88\u626b\u63cf",
  lanScanNoInterface: "\u6ca1\u6709\u53ef\u7528\u7684\u5c40\u57df\u7f51\u7f51\u5361\u3002",
  lanScanInvalidSubnet: "\u7f51\u6bb5\u65e0\u6548\uff0c\u4ec5\u652f\u6301 /24\u3002",
  lanScanNoSubnet: "\u8bf7\u9009\u62e9\u8981\u626b\u63cf\u7684\u7f51\u6bb5\u3002",
  lanTransferAddDevice: "\u6dfb\u52a0",
  lanTransferIpPlaceholder:
    "\u8f93\u5165\u5bf9\u7aef IP\uff08\u5982 192.168.1.20\uff09",
  lanTransferNoDevices:
    "\u6682\u65f6\u6ca1\u6709\u53d1\u73b0\u8bbe\u5907\uff0c\u8bf7\u786e\u4fdd\u5bf9\u7aef LocalSend \u5df2\u6253\u5f00\u63a5\u6536",
  lanTransferTrusted: "\u5df2\u4fe1\u4efb",
  lanTransferSendFile: "\u53d1\u9001\u6587\u4ef6",
  lanTransferSendText: "\u53d1\u9001\u6587\u672c",
  lanTransferSendTextTitle: "\u53d1\u9001\u6587\u672c\u7ed9 {name}",
  lanTransferSendTextPlaceholder:
    "\u8f93\u5165\u8981\u53d1\u9001\u7684\u6587\u672c\uff0c\u5bf9\u7aef\u4f1a\u76f4\u63a5\u663e\u793a\u4e3a\u6d88\u606f",
  lanTransferTransfers: "\u4f20\u8f93\u8bb0\u5f55",
  lanTransferReceived: "\u5df2\u6536\u5230\u7684\u6587\u4ef6",
  lanTransferCancel: "\u53d6\u6d88",
  lanTransferWebShare: "\u5206\u4eab\u7ed9\u6d4f\u89c8\u5668",
  lanTransferWebReceive: "\u6d4f\u89c8\u5668\u4f20\u7ed9\u6211",
  lanTransferWebShareHint:
    "\u540c\u4e00\u5c40\u57df\u7f51\u5185\u7684\u6d4f\u89c8\u5668\u6253\u5f00\u540e\u53ef\u4e0b\u8f7d\u672c\u673a\u5206\u4eab\u7684\u6587\u4ef6\uff0c\u6587\u672c\u4f1a\u76f4\u63a5\u5c55\u793a\u3002",
  lanTransferWebReceiveHint:
    "\u6d4f\u89c8\u5668\u6253\u5f00\u540e\u53ef\u4e0a\u4f20\u6587\u4ef6\uff0c\u4e5f\u53ef\u8f93\u5165\u6587\u672c\u53d1\u9001\u5230\u672c\u673a\u526a\u8d34\u677f\u3002",
  lanTransferWebIdleHint:
    "\u9009\u62e9\u4e00\u4e2a\u6a21\u5f0f\u540e\u751f\u6210\u624b\u673a\u626b\u7801\u5730\u5740\uff1a\u4e24\u79cd\u6a21\u5f0f\u4e92\u65a5\uff0c\u5207\u6362\u65f6\u4f1a\u91cd\u542f\u670d\u52a1\u3002",
  lanTransferWebStop: "\u5173\u95ed\u626b\u7801\u9875",
  lanTransferStartService: "\u542f\u52a8\u670d\u52a1",
  lanTransferStopService: "\u505c\u6b62\u670d\u52a1",
  lanTransferDisabledHint:
    "\u5c40\u57df\u7f51\u4e92\u4f20\u5df2\u5728\u8bbe\u7f6e\u4e2d\u5173\u95ed\uff0c\u53ef\u5728\u8bbe\u7f6e\u9875\u201c\u4e92\u4f20\u201d\u5206\u7ec4\u91cd\u65b0\u5f00\u542f\u3002",
  lanIncomingTitle: "\u6536\u5230\u4f20\u8f93\u8bf7\u6c42",
  lanIncomingAccept: "\u63a5\u53d7",
  lanIncomingDecline: "\u62d2\u7edd",
  lanIncomingAcceptAndTrust: "\u63a5\u53d7\u5e76\u4fe1\u4efb",
  lanTransferTitle: "LocalSend",
  lanTransferDropFiles: "\u677e\u5f00\u4ee5\u53d1\u9001\u6587\u4ef6",
  windowControlStyle: "\u7a97\u53e3\u63a7\u4ef6\u6837\u5f0f",
  windowControlStyleTip: "\u4ec5\u5f71\u54cd\u5f53\u524d\u5e94\u7528\u7684\u6807\u9898\u680f\u6309\u94ae\uff0c\u5207\u6362\u540e\u7acb\u5373\u751f\u6548\u3002",
  windowControlTrafficLights: "\u7ea2\u7eff\u706f",
  windowControlWindows: "Windows",
  minimizeAction: "\u6700\u5c0f\u5316",
  maximizeAction: "\u6700\u5927\u5316",
  restoreAction: "\u8fd8\u539f",
  lanTransferSend: "\u53d1\u9001",
  openAction: "\u6253\u5f00",
  revealInExplorer:
    "\u5728\u6587\u4ef6\u7ba1\u7406\u5668\u4e2d\u663e\u793a",
  lanTransferDownloadDirMissing:
    "\u4e92\u4f20\u6587\u4ef6\u4fdd\u5b58\u76ee\u5f55\u4e0d\u5b58\u5728",
  lanTransferDownloadDirNotDirectory:
    "\u4e92\u4f20\u6587\u4ef6\u4fdd\u5b58\u4f4d\u7f6e\u4e0d\u662f\u76ee\u5f55",
  lanTransferDownloadDirNotWritable:
    "\u4e92\u4f20\u6587\u4ef6\u4fdd\u5b58\u76ee\u5f55\u4e0d\u53ef\u5199",
  lanTransferPin: "\u63a5\u6536 PIN",
  lanTransferPinTip:
    "\u8bbe\u7f6e\u540e\uff0c\u5176\u4ed6\u8bbe\u5907\u5fc5\u987b\u8f93\u5165\u76f8\u540c PIN \u624d\u80fd\u4f20\u8f93\uff1b\u7559\u7a7a\u8868\u793a\u4e0d\u9700\u8981\u3002",
  lanTransferPinPlaceholder: "\u7559\u7a7a\u8868\u793a\u4e0d\u9700\u8981",
  lanPinTitle: "\u8f93\u5165 PIN",
  lanPinHint:
    "\u8be5\u8bbe\u5907\u542f\u7528\u4e86 PIN\uff0c\u8bf7\u8f93\u5165\u540e\u7ee7\u7eed\u3002",
  lanPinPlaceholder: "PIN",
  lanPinInvalid: "PIN \u4e0d\u6b63\u786e\uff0c\u8bf7\u91cd\u65b0\u8f93\u5165\u3002",
  lanErrorPortInUse:
    "\u7aef\u53e3 53317 \u5df2\u88ab\u5360\u7528\uff0c\u672c\u673a\u53ef\u80fd\u6709\u5176\u5b83 LocalSend \u5ba2\u6237\u7aef\u6b63\u5728\u8fd0\u884c\u3002",
  lanErrorMulticastUnavailable:
    "\u7ec4\u64ad\u4e0d\u53ef\u7528\uff0c\u8bbe\u5907\u53d1\u73b0\u53ef\u80fd\u5931\u8d25\uff1b\u53ef\u624b\u52a8\u6dfb\u52a0\u8bbe\u5907\u8fde\u63a5\u3002",
  lanWarningMulticastWindows:
    "\u7ec4\u64ad\u4e0d\u53ef\u7528\uff0c\u8bbe\u5907\u53d1\u73b0\u53ef\u80fd\u5931\u8d25\uff1b\u8bf7\u68c0\u67e5 Windows \u9632\u706b\u5899\u662f\u5426\u653e\u884c Power Paste\uff0c\u6216\u624b\u52a8\u6dfb\u52a0\u8bbe\u5907\u8fde\u63a5\u3002",
  lanWarningMulticastMacos:
    "\u7ec4\u64ad\u4e0d\u53ef\u7528\uff0c\u8bbe\u5907\u53d1\u73b0\u53ef\u80fd\u5931\u8d25\uff1b\u8bf7\u5728\u300c\u7cfb\u7edf\u8bbe\u7f6e \u2192 \u9690\u79c1\u4e0e\u5b89\u5168\u6027 \u2192 \u672c\u5730\u7f51\u7edc\u300d\u4e2d\u5141\u8bb8 Power Paste\uff0c\u6216\u624b\u52a8\u6dfb\u52a0\u8bbe\u5907\u8fde\u63a5\u3002",
  lanWarningMulticastLinux:
    "\u7ec4\u64ad\u4e0d\u53ef\u7528\uff0c\u8bbe\u5907\u53d1\u73b0\u53ef\u80fd\u5931\u8d25\uff1b\u8bf7\u68c0\u67e5\u9632\u706b\u5899\uff08ufw/firewalld\uff09\u662f\u5426\u653e\u884c TCP 53317 \u4e0e UDP \u7ec4\u64ad 224.0.0.167:53317\uff0c\u6216\u624b\u52a8\u6dfb\u52a0\u8bbe\u5907\u8fde\u63a5\u3002",
  lanErrorListenerFailed:
    "\u5c40\u57df\u7f51\u76d1\u542c\u5931\u8d25\uff0c\u5df2\u5c1d\u8bd5\u81ea\u52a8\u6062\u590d\uff1b\u82e5\u4ecd\u5931\u8d25\u8bf7\u91cd\u542f\u670d\u52a1\u3002",
  lanErrorServiceFailed: "\u5c40\u57df\u7f51\u670d\u52a1\u542f\u52a8\u5931\u8d25\u3002",
  lanErrorNotRunning: "\u5c40\u57df\u7f51\u670d\u52a1\u672a\u8fd0\u884c\u3002",
  lanErrorDeviceMissing:
    "\u627e\u4e0d\u5230\u8be5\u8bbe\u5907\uff0c\u8bf7\u5237\u65b0\u8bbe\u5907\u5217\u8868\u540e\u91cd\u8bd5\u3002",
  lanErrorNoFiles: "\u6ca1\u6709\u53ef\u53d1\u9001\u7684\u6587\u4ef6\u3002",
  lanErrorFileNotFound: "\u6587\u4ef6\u4e0d\u5b58\u5728\u6216\u5df2\u88ab\u79fb\u52a8\u3002",
  lanErrorRequestMissing: "\u8be5\u8bf7\u6c42\u5df2\u5931\u6548\u3002",
  lanErrorPinRequired:
    "\u5bf9\u7aef\u542f\u7528\u4e86 PIN\uff0c\u8bf7\u8f93\u5165 PIN \u540e\u91cd\u8bd5\u3002",
  lanErrorDeclined: "\u5bf9\u65b9\u62d2\u7edd\u4e86\u672c\u6b21\u4f20\u8f93\u3002",
  lanErrorBusy:
    "\u5bf9\u65b9\u6b63\u5728\u5904\u7406\u5176\u5b83\u4f20\u8f93\uff0c\u8bf7\u7a0d\u540e\u518d\u8bd5\u3002",
  lanErrorTooManyRequests:
    "PIN \u5c1d\u8bd5\u6b21\u6570\u8fc7\u591a\uff0c\u5bf9\u7aef\u5df2\u6682\u65f6\u62d2\u7edd\u672c\u673a\uff1b\u8bf7\u5728\u5bf9\u7aef\u91cd\u65b0\u5f00\u59cb\u63a5\u6536\u6216\u91cd\u542f\u5bf9\u7aef\u5e94\u7528\u540e\u518d\u8bd5\u3002",
  lanErrorEmptyText: "\u8bf7\u8f93\u5165\u8981\u53d1\u9001\u7684\u6587\u672c\u3002",
  lanErrorGeneric: "\u4f20\u8f93\u5931\u8d25\uff1a{detail}",
  settingsCategorySync: "\u540c\u6b65",
  webdavSyncEnabled: "WebDAV \u540c\u6b65",
  webdavAutoSync: "\u81ea\u52a8\u540c\u6b65",
  webdavServerUrl: "WebDAV \u5730\u5740",
  webdavServerUrlTip:
    "\u586b\u5199 WebDAV \u670d\u52a1\u6839\u5730\u5740\uff0c\u5efa\u8bae\u4f7f\u7528 HTTPS\u3002",
  webdavUsername: "\u7528\u6237\u540d",
  webdavPassword: "\u5bc6\u7801",
  webdavPasswordPlaceholder: "\u7559\u7a7a\u8868\u793a\u4e0d\u66f4\u6539",
  webdavPasswordSaved: "\u5df2\u4fdd\u5b58\u5230\u7cfb\u7edf\u51ed\u636e",
  webdavPasswordSavedPlaceholder:
    "\u5bc6\u7801\u5df2\u4fdd\u5b58\uff0c\u8f93\u5165\u65b0\u5bc6\u7801\u53ef\u8986\u76d6",
  webdavRemoteDir: "\u8fdc\u7a0b\u76ee\u5f55",
  webdavSyncStatus: "\u540c\u6b65\u72b6\u6001",
  webdavLastSyncAt: "\u4e0a\u6b21\u540c\u6b65\uff1a{time}",
  webdavNeverSynced: "\u5c1a\u672a\u540c\u6b65",
  webdavTestConnection: "\u6d4b\u8bd5\u8fde\u63a5",
  webdavSyncNow: "\u7acb\u5373\u540c\u6b65",
  webdavSyncing: "\u540c\u6b65\u4e2d",
  webdavClearPassword: "\u6e05\u9664\u5bc6\u7801",
  webdavSyncFailed: "WebDAV \u540c\u6b65\u5931\u8d25",
  webdavSettingsIncomplete:
    "\u8bf7\u5148\u586b\u5199 WebDAV \u5730\u5740\u548c\u7528\u6237\u540d",
  webdavCredentialMissing:
    "\u8bf7\u5148\u586b\u5199\u5e76\u4fdd\u5b58 WebDAV \u5bc6\u7801",
  webdavConnectionFailed:
    "WebDAV \u8fde\u63a5\u5931\u8d25\uff0c\u8bf7\u68c0\u67e5\u5730\u5740\u3001\u7528\u6237\u540d\u548c\u5bc6\u7801",
  webdavUnauthorized:
    "WebDAV \u8ba4\u8bc1\u5931\u8d25\uff08401\uff09\uff0c\u575a\u679c\u4e91\u9700\u4f7f\u7528\u5e94\u7528\u5bc6\u7801\uff0c\u4e0d\u662f\u767b\u5f55\u5bc6\u7801",
  webdavForbidden:
    "WebDAV \u6743\u9650\u4e0d\u8db3\uff08403\uff09\uff0c\u8bf7\u68c0\u67e5\u8d26\u53f7\u6743\u9650\u548c\u8fdc\u7a0b\u76ee\u5f55",
  webdavNotFound:
    "WebDAV \u5730\u5740\u4e0d\u5b58\u5728\uff08404\uff09\uff0c\u575a\u679c\u4e91\u901a\u5e38\u4f7f\u7528 https://dav.jianguoyun.com/dav/",
  webdavMethodNotAllowed:
    "WebDAV \u65b9\u6cd5\u4e0d\u88ab\u8be5\u8def\u5f84\u652f\u6301\uff08405\uff09\uff0c\u8bf7\u68c0\u67e5\u5730\u5740\u662f\u5426\u4e3a WebDAV \u6839\u5730\u5740",
  webdavServiceUnavailable:
    "WebDAV \u670d\u52a1\u6682\u65f6\u4e0d\u53ef\u7528\uff08503\uff09\uff0c\u53ef\u80fd\u662f\u670d\u52a1\u7aef\u9650\u6d41\u6216\u6b63\u5728\u5904\u7406\u5927\u91cf\u6587\u4ef6\uff0c\u8bf7\u7a0d\u540e\u518d\u8bd5",
  webdavEndpointGone:
    "WebDAV \u5730\u5740\u8fd4\u56de 410 Gone\uff0c\u8bf7\u68c0\u67e5\u670d\u52a1\u5730\u5740\u662f\u5426\u6b63\u786e\u3002\u575a\u679c\u4e91\u901a\u5e38\u4f7f\u7528 https://dav.jianguoyun.com/dav/",
  webdavRemoteCleanupFailed:
    "\u5185\u5bb9\u5df2\u4e0a\u4f20\uff0c\u4f46\u8fdc\u7a0b\u5220\u9664\u6807\u8bb0\u6e05\u7406\u5931\u8d25\uff0c\u8bf7\u518d\u6b21\u540c\u6b65",
  webdavManifestSaveFailed:
    "\u5185\u5bb9\u5df2\u4e0a\u4f20\uff0c\u4f46\u540c\u6b65\u7d22\u5f15\u4fdd\u5b58\u5931\u8d25\uff0c\u8bf7\u518d\u6b21\u540c\u6b65",
  webdavManifestFetchFailed:
    "\u8bfb\u53d6 WebDAV \u540c\u6b65\u7d22\u5f15\u5931\u8d25\uff0c\u8bf7\u518d\u8bd5",
  webdavItemUploadFailed:
    "\u4e0a\u4f20\u5386\u53f2\u6761\u76ee\u5931\u8d25\uff0c\u8bf7\u68c0\u67e5 WebDAV \u7a7a\u95f4\u548c\u7f51\u7edc",
  webdavItemDownloadFailed:
    "\u4e0b\u8f7d\u8fdc\u7a0b\u5386\u53f2\u6761\u76ee\u5931\u8d25\uff0c\u8bf7\u518d\u8bd5",
  webdavFolderCreateFailed:
    "\u521b\u5efa WebDAV \u540c\u6b65\u76ee\u5f55\u5931\u8d25\uff0c\u8bf7\u68c0\u67e5\u8fdc\u7a0b\u76ee\u5f55\u6743\u9650",
});

Object.assign(messages["en-US"], {
  unsupportedCurrentPlatform:
    "This action is not available on the current platform.",
  unsupportedClipboardWrite:
    "Writing back to the system clipboard is not available on this platform.",
  unsupportedDirectPaste:
    "Direct paste into the target app is not available on this platform.",
  linuxX11ToolsMissing:
    "Direct paste on Linux (X11) requires xdotool. Example installs: `sudo apt install xdotool`, `sudo dnf install xdotool`, or `sudo pacman -S xdotool`, then try again.",
  linuxWaylandToolsMissing:
    "Direct paste on Linux (Wayland) requires wtype. Example installs: `sudo apt install wtype`, `sudo dnf install wtype`, or `sudo pacman -S wtype`, then try again.",
  pasteTargetFocusFailed:
    "The target window could not be focused. Paste was cancelled.",
  pasteTargetPermissionDenied:
    "Power Paste does not have the required Accessibility or Automation permission. Allow it in System Settings > Privacy & Security > Accessibility / Automation and try again.",
  unsupportedLaunchOnStartup:
    "Launch on startup is not available on this platform.",
  duplicateShortcut: "Shortcuts must be unique.",
  invalidShortcut: "The shortcut format is invalid.",
  shortcutRegistrationFailed:
    "The shortcut is not active. It may already be used by another app.",
  shortcutConflictMessage:
    "{name} ({shortcut}) is not active. It may already be used by another app. Close the conflicting app and retry, or choose a different shortcut in Settings.",
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
  closeAction: "Close",
  lanReceiverTitle: "LocalSend",
  lanTransferStatusRunning: "Running",
  lanTransferStatusStopped: "Stopped",
  lanTransferStatusPortInUse:
    "Port 53317 is in use. LocalSend may already be running on this device.",
  lanTransferStatusActive: "Transferring",
  lanTransferStatusDone: "Completed",
  lanTransferStatusFailed: "Failed",
  lanTransferStatusCancelled: "Cancelled",
  lanTransferDevices: "Nearby devices",
  lanTransferRefresh: "Refresh",
  lanScanMenuTitle: "Choose a subnet",
  lanScanMenuHint:
    "Only the selected subnet is scanned; other subnets are left alone.",
  lanScanLastUsed: "Last",
  lanScanCancel: "Cancel scan",
  lanScanNoInterface: "No usable LAN interface was found.",
  lanScanInvalidSubnet: "Invalid subnet: only /24 is supported.",
  lanScanNoSubnet: "Choose a subnet to scan.",
  lanTransferAddDevice: "Add",
  lanTransferIpPlaceholder: "Enter the peer IP (e.g. 192.168.1.20)",
  lanTransferNoDevices:
    "No device discovered yet. Make sure LocalSend is receiving on the other device.",
  lanTransferTrusted: "Trusted",
  lanTransferSendFile: "Send files",
  lanTransferSendText: "Send text",
  lanTransferSendTextTitle: "Send text to {name}",
  lanTransferSendTextPlaceholder:
    "Type the text to send; it shows up as a message on the other device",
  lanTransferTransfers: "Transfers",
  lanTransferReceived: "Received files",
  lanTransferCancel: "Cancel",
  lanTransferWebShare: "Share to a browser",
  lanTransferWebReceive: "Upload from a browser",
  lanTransferWebShareHint:
    "Open the link in any browser on the same LAN to download what this device shares; text is shown inline.",
  lanTransferWebReceiveHint:
    "Open the link in a browser to upload files or type text that lands in this device's clipboard.",
  lanTransferWebIdleHint:
    "Pick a mode to get a QR code: the download and upload pages are mutually exclusive, and switching restarts the service.",
  lanTransferWebStop: "Stop link",
  lanTransferStartService: "Start service",
  lanTransferStopService: "Stop service",
  lanTransferDisabledHint:
    "LAN transfer is turned off in settings. Re-enable it under the Transfer category.",
  lanIncomingTitle: "Incoming transfer",
  lanIncomingAccept: "Accept",
  lanIncomingDecline: "Decline",
  lanIncomingAcceptAndTrust: "Accept and trust",
  lanTransferTitle: "LocalSend",
  lanTransferDropFiles: "Drop files to send",
  windowControlStyle: "Window controls",
  windowControlStyleTip: "Changes the app title bar buttons immediately.",
  windowControlTrafficLights: "Traffic lights",
  windowControlWindows: "Windows",
  minimizeAction: "Minimize",
  maximizeAction: "Maximize",
  restoreAction: "Restore down",
  lanTransferSend: "Send",
  openAction: "Open",
  revealInExplorer: "Show in File Manager",
  lanTransferDownloadDirMissing: "The transfer download folder does not exist.",
  lanTransferDownloadDirNotDirectory:
    "The transfer download location is not a folder.",
  lanTransferDownloadDirNotWritable:
    "The transfer download folder is not writable.",
  lanTransferPin: "Receive PIN",
  lanTransferPinTip:
    "When set, other devices must enter the same PIN to transfer. Leave blank to disable.",
  lanTransferPinPlaceholder: "Leave blank to disable",
  lanPinTitle: "Enter PIN",
  lanPinHint: "This device requires a PIN. Enter it to continue.",
  lanPinPlaceholder: "PIN code",
  lanPinInvalid: "Incorrect PIN. Please try again.",
  lanErrorPortInUse:
    "Port 53317 is already in use. Another LocalSend client may be running on this machine.",
  lanErrorMulticastUnavailable:
    "Multicast is unavailable, so discovery may fail. Add a device manually to connect.",
  lanWarningMulticastWindows:
    "Multicast is unavailable, so discovery may fail. Check that the Windows firewall allows Power Paste, or add the device manually.",
  lanWarningMulticastMacos:
    "Multicast is unavailable, so discovery may fail. Allow Power Paste under System Settings → Privacy & Security → Local Network, or add the device manually.",
  lanWarningMulticastLinux:
    "Multicast is unavailable, so discovery may fail. Check that the firewall (ufw/firewalld) allows TCP 53317 and UDP multicast 224.0.0.167:53317, or add the device manually.",
  lanErrorListenerFailed:
    "The LAN listener failed. An automatic recovery was attempted; restart the service if it keeps failing.",
  lanErrorServiceFailed: "Could not start the LAN service.",
  lanErrorNotRunning: "The LAN service is not running.",
  lanErrorDeviceMissing: "Device not found. Refresh the device list and try again.",
  lanErrorNoFiles: "No files to send.",
  lanErrorFileNotFound: "The file no longer exists.",
  lanErrorRequestMissing: "This request has expired.",
  lanErrorPinRequired: "The peer requires a PIN. Enter it and try again.",
  lanErrorDeclined: "The peer declined the transfer.",
  lanErrorBusy: "The peer is busy with another transfer. Try again later.",
  lanErrorTooManyRequests:
    "Too many PIN attempts; the peer has temporarily blocked this device. Restart receiving (or the peer app) and try again.",
  lanErrorEmptyText: "Enter the text to send.",
  lanErrorGeneric: "Transfer failed: {detail}",
  settingsCategorySync: "Sync",
  webdavSyncEnabled: "WebDAV Sync",
  webdavAutoSync: "Auto sync",
  webdavServerUrl: "WebDAV URL",
  webdavServerUrlTip:
    "Enter the WebDAV service root URL. HTTPS is recommended.",
  webdavUsername: "Username",
  webdavPassword: "Password",
  webdavPasswordPlaceholder: "Leave blank to keep current password",
  webdavPasswordSaved: "Saved to system credentials",
  webdavPasswordSavedPlaceholder:
    "Password saved. Enter a new one to replace it",
  webdavRemoteDir: "Remote folder",
  webdavSyncStatus: "Sync status",
  webdavLastSyncAt: "Last sync: {time}",
  webdavNeverSynced: "Not synced yet",
  webdavTestConnection: "Test connection",
  webdavSyncNow: "Sync now",
  webdavSyncing: "Syncing",
  webdavClearPassword: "Clear password",
  webdavSyncFailed: "WebDAV sync failed",
  webdavSettingsIncomplete: "Enter the WebDAV URL and username first.",
  webdavCredentialMissing: "Enter and save the WebDAV password first.",
  webdavConnectionFailed:
    "WebDAV connection failed. Check the URL, username, and password.",
  webdavUnauthorized:
    "WebDAV authentication failed (401). Jianguoyun requires an app password, not the login password.",
  webdavForbidden:
    "WebDAV permission denied (403). Check account permissions and the remote folder.",
  webdavNotFound:
    "WebDAV URL not found (404). Jianguoyun usually uses https://dav.jianguoyun.com/dav/.",
  webdavMethodNotAllowed:
    "This WebDAV method is not allowed on that path (405). Check that the URL is the WebDAV root.",
  webdavServiceUnavailable:
    "The WebDAV service is temporarily unavailable (503). It may be rate-limited or processing many files. Try again later.",
  webdavEndpointGone:
    "The WebDAV URL returned 410 Gone. Check the service URL. Jianguoyun usually uses https://dav.jianguoyun.com/dav/.",
  webdavRemoteCleanupFailed:
    "Content was uploaded, but remote deletion cleanup failed. Sync again.",
  webdavManifestSaveFailed:
    "Content was uploaded, but the sync index could not be saved. Sync again.",
  webdavManifestFetchFailed: "Failed to read the WebDAV sync index. Try again.",
  webdavItemUploadFailed:
    "Failed to upload a history item. Check WebDAV storage and network.",
  webdavItemDownloadFailed:
    "Failed to download a remote history item. Try again.",
  webdavFolderCreateFailed:
    "Failed to create the WebDAV sync folder. Check remote folder permissions.",
});


Object.assign(messages["zh-CN"], {
  addAction: "添加",
  addIgnoredApp: "添加应用",
  ignoredAppAdded: "已添加",
  ignoredAppsEmpty: "尚未忽略任何应用",
  ignoredAppsTip: "从这些应用复制内容时不会写入历史记录，不影响系统剪贴板本身。",
  ignoredAppsLinuxUnsupported: "当前版本暂不支持枚举 Linux 已安装应用，后续版本补充。",
  ignoredAppPickerLoadFailed: "加载已安装应用失败",
  loadingApps: "正在加载应用...",
  noAppsFound: "未找到应用",
  removeAction: "移除",
  searchAppsPlaceholder: "搜索应用",
});

Object.assign(messages["en-US"], {
  addAction: "Add",
  addIgnoredApp: "Add app",
  ignoredAppAdded: "Added",
  ignoredAppsEmpty: "No ignored apps yet",
  ignoredAppsTip:
    "Copies from these apps are not saved to history. The system clipboard itself is unchanged.",
  ignoredAppsLinuxUnsupported:
    "This version cannot enumerate installed Linux apps yet. Support will be added later.",
  ignoredAppPickerLoadFailed: "Failed to load installed apps",
  loadingApps: "Loading apps...",
  noAppsFound: "No apps found",
  removeAction: "Remove",
  searchAppsPlaceholder: "Search apps",
});

Object.assign(messages["zh-CN"], {
  confirmAction: "确认",
  lanTransferReceive: "接收",
  lanTransferSend: "发送",
  lanTransferHistory: "传输历史",
  lanTransferAdvanced: "高级信息",
  lanTransferSelectionTitle: "选择",
  lanTransferSelectionClear: "清空选择",
  lanTransferSelectionCount: "已选 {count} 个文件",
  lanTransferPickFile: "文件",
  lanTransferPickFolder: "文件夹",
  lanTransferPickText: "文本",
  lanTransferPickClipboard: "剪贴板",
  lanTransferTextMessage: "文本消息",
  lanTransferTextSelectionHint: "点击文本条目可修改内容。",
  lanTransferReceiveLink: "通过链接接收",
  lanTransferDeviceName: "设备名称",
  lanTransferLocalIp: "本机 IP",
  lanTransferUnknownIp: "暂未获取到 IP",
  lanTransferPort: "端口",
  lanTransferUnknownDevice: "未知设备",
  lanTransferUnknownModel: "未知型号",
  lanTransferManualSend: "手动发送",
  lanTransferSendHelp: "请确保接收设备连接到同一局域网。",
  lanTransferReceivedEmpty: "暂无接收记录。",
  lanTransferTransfersEmpty: "暂无传输记录。",
  lanTransferWebOpen: "打开",
  lanTransferWebStarting: "正在启动浏览器互传服务...",
  lanErrorSelectionPathNotFound: "选择的文件或文件夹不存在。",
  lanErrorSelectionPathNotReadable: "选择的文件或文件夹无法读取。",
  lanErrorSelectionSymlinkUnsupported: "暂不支持发送符号链接。",
  lanErrorClipboardEmpty: "剪贴板中没有可发送的内容。",
  lanErrorClipboardWriteFailed: "无法准备剪贴板中的图片。",
  lanErrorClipboardUnavailable:
    "当前会话无法访问系统剪贴板：Linux 需要 X11 / XWayland，或支持 wlr-data-control 的 Wayland 合成器。",
});

Object.assign(messages["en-US"], {
  confirmAction: "Confirm",
  lanTransferReceive: "Receive",
  lanTransferSend: "Send",
  lanTransferHistory: "Transfer history",
  lanTransferAdvanced: "Advanced info",
  lanTransferSelectionTitle: "Selection",
  lanTransferSelectionClear: "Clear",
  lanTransferSelectionCount: "Files: {count}",
  lanTransferPickFile: "File",
  lanTransferPickFolder: "Folder",
  lanTransferPickText: "Text",
  lanTransferPickClipboard: "Paste",
  lanTransferTextMessage: "Text message",
  lanTransferTextSelectionHint: "Select the text item to edit its content.",
  lanTransferReceiveLink: "Receive via link",
  lanTransferDeviceName: "Device name",
  lanTransferLocalIp: "Local IP",
  lanTransferUnknownIp: "No IP address found",
  lanTransferPort: "Port",
  lanTransferUnknownDevice: "Unknown device",
  lanTransferUnknownModel: "Unknown model",
  lanTransferManualSend: "Manual sending",
  lanTransferSendHelp: "Make sure the receiving device is on the same LAN.",
  lanTransferReceivedEmpty: "No received files yet.",
  lanTransferTransfersEmpty: "No transfer history yet.",
  lanTransferWebOpen: "Open",
  lanTransferWebStarting: "Starting the browser transfer service...",
  lanErrorSelectionPathNotFound: "The selected file or folder does not exist.",
  lanErrorSelectionPathNotReadable: "The selected file or folder cannot be read.",
  lanErrorSelectionSymlinkUnsupported: "Sending symbolic links is not supported.",
  lanErrorClipboardEmpty: "There is nothing in the clipboard to send.",
  lanErrorClipboardWriteFailed: "Could not prepare the clipboard image.",
  lanErrorClipboardUnavailable:
    "The system clipboard is unavailable in this session: Linux needs X11 / XWayland, or a Wayland compositor that supports wlr-data-control.",
});

export function translate(locale, key, params = {}) {
  const pack = messages[locale] || messages[defaultLocale];
  const fallback = messages["en-US"];
  const template = pack[key] ?? fallback[key] ?? key;
  return template.replace(/\{(\w+)\}/g, (_, name) => `${params[name] ?? ""}`);
}
