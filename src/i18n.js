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
  debugMode: "调试模式",
  hardwareAcceleration: "硬件加速",
  hardwareAccelerationTip:
    "关闭后会让 WebView2 以禁用 GPU 加速参数启动，可能降低内存占用，也可能影响渲染性能。重启应用后生效。",
  toggleOn: "开启",
  toggleOff: "关闭",
});

localeOptions[0].label = "简体中文";

densityOptions["zh-CN"] = [
  { value: "compact", label: "紧凑" },
  { value: "cozy", label: "舒适" },
];

themeModeOptions["zh-CN"] = [
  { value: "light", label: "浅色" },
  { value: "dark", label: "深色" },
  { value: "system", label: "跟随系统" },
];

accentColorOptions["zh-CN"] = [
  { value: "ocean", label: "海蓝" },
  { value: "amber", label: "琥珀" },
  { value: "jade", label: "青玉" },
  { value: "rose", label: "玫瑰" },
];

Object.assign(messages["zh-CN"], {
  unsupportedCurrentPlatform:
    "当前平台暂不支持该操作",
  unsupportedClipboardWrite:
    "当前平台暂不支持写入系统剪贴板",
  unsupportedDirectPaste:
    "当前平台暂不支持直接粘贴到目标应用",
  linuxX11ToolsMissing:
    "Linux（X11）直接粘贴需要额外安装 xdotool。可参考：Ubuntu/Debian 执行 `sudo apt install xdotool`，Fedora 执行 `sudo dnf install xdotool`，Arch 执行 `sudo pacman -S xdotool`，安装后重试即可。",
  linuxWaylandToolsMissing:
    "Linux（Wayland）直接粘贴需要 wtype、ydotool 或桌面提供的远程输入门户（RemoteDesktop）。可参考安装：Ubuntu/Debian 执行 `sudo apt install wtype`，Fedora 执行 `sudo dnf install wtype`，Arch 执行 `sudo pacman -S wtype`；GNOME / KDE 的合成器不实现虚拟键盘协议（wtype 会提示 “Compositor does not support the virtual keyboard protocol”），需要在系统弹窗中允许远程输入，或改用 ydotool（内核 uinput，需要自行配置 ydotoold）。",
  pasteTargetFocusFailed:
    "未能恢复目标窗口焦点，已取消本次粘贴",
  pasteTargetPermissionDenied:
    "系统未授予 Power Paste 辅助功能或自动化权限，请在“系统设置 > 隐私与安全性 > 辅助功能 / 自动化”中允许后重试",
  pastePortalDenied:
    "桌面没有允许远程输入，自动粘贴被系统拒绝。请在系统授权窗口里打开「Allow Remote Interaction」再确认，或改为复制到剪贴板后手动粘贴。",
  pastePortalUnavailable:
    "当前 Wayland 桌面既没有实现 wtype 依赖的虚拟键盘协议，也没有提供远程输入门户（RemoteDesktop），无法自动发送粘贴快捷键。可改用 ydotool（内核 uinput，需要自行配置 ydotoold），或在系统设置里改为复制到剪贴板后手动粘贴。",
  pastePortalFailed:
    "桌面远程输入门户发送粘贴快捷键失败，请重试；若持续失败，可在系统设置里改为复制到剪贴板后手动粘贴。",
  pasteAuthorizationPending:
    "首次自动粘贴需要授权：请在系统弹出的 “Remote Desktop” 窗口中打开 “Allow Remote Interaction”，再点击 “Share”。授权在本次运行内复用，不会反复弹窗。",
  pastePortalTimeout:
    "等待系统授权超时，本次自动粘贴已取消。请重试，并在弹出的 “Remote Desktop” 窗口里打开 “Allow Remote Interaction” 后点击 “Share”。",
  unsupportedLaunchOnStartup:
    "当前平台暂不支持开机启动",
  duplicateShortcut: "快捷键不能重复",
  invalidShortcut: "快捷键格式不正确",
  shortcutRegistrationFailed:
    "快捷键未生效，可能已被其他程序占用",
  shortcutConflictMessage:
    "{name}（{shortcut}）未生效，可能已被其他程序占用。请关闭占用程序后重试，或在设置中更换快捷键。",
  waylandPortalUnavailable:
    "当前 Wayland 桌面没有提供全局快捷键门户（GlobalShortcuts），无法自动绑定快捷键。可在系统设置里为 Power Paste 手动指定快捷键，或改用 X11 会话。",
  waylandPortalDenied:
    "桌面环境未绑定该快捷键：可能是系统弹窗被取消，或该快捷键与系统快捷键冲突。请重试并在弹窗中确认，或改用其它按键。",
  waylandPortalFailed:
    "桌面环境绑定全局快捷键失败，请重试。若持续失败，可在系统设置里手动为 Power Paste 指定快捷键。",
  waylandPortalClosed:
    "桌面环境已释放快捷键绑定（例如桌面会话重启），请重试以重新绑定。",
  waylandPortalNoAppId:
    "桌面无法识别当前进程的应用标识，因此拒绝绑定快捷键。请从应用列表启动 Power Paste（本地构建先执行 pnpm desktop:install），或在项目目录用 pnpm tauri dev 启动。",
  waylandPortalInvalidAppId:
    "当前进程的应用标识「{appId}」不符合桌面要求：GNOME 只接受反向域名格式（至少包含一个点，例如 com.yulei.powerpaste），因此绑定请求被系统直接丢弃。请从应用列表启动 Power Paste（本地构建先执行 pnpm desktop:install），或在项目目录用 pnpm tauri dev 启动。",
  checkForUpdates: "检查更新",
  downloadAndInstall: "下载并安装",
  updateIdle: "尚未执行更新检查",
  checkingForUpdates: "正在检查更新",
  updateAvailable: "发现新版本",
  updateAvailableVersion: "发现新版本 {version}",
  updateConfirmInstall:
    "发现新版本，是否立即下载并安装？",
  updateConfirmInstallVersion:
    "发现新版本 {version}，是否立即下载并安装？",
  downloadingUpdate: "正在下载更新",
  downloadingUpdateProgress: "正在下载更新 {percent}%",
  updateReadyToInstall:
    "更新已下载，安装程序即将启动",
  upToDate: "当前已是最新版本",
  updateCheckFailed: "检查更新失败",
  updateInstallFailed: "安装更新失败",
  currentVersionLabel: "当前版本：{version}",
  latestVersionLabel: "最新版本：{version}",
  updateDetailsTitle: "更新内容",
  updateNotesEmpty: "暂无更新说明",
  ignoreUpdate: "忽略",
  installUpdateNow: "安装",
  updateDebugTitle: "更新调试",
  updateDebugHint:
    "仅开发模式可见，用于预览 new 徽标和更新状态。",
  updateDebugVersionLabel: "调试版本号",
  updateDebugVersionPlaceholder: "例如 v0.3.6",
  updateDebugBodyLabel: "调试更新说明",
  updateDebugBodyPlaceholder:
    "在这里输入 Markdown 更新说明",
  updateDebugAvailable: "新版本",
  updateDebugDownloading: "下载中",
  updateDebugDownloaded: "已下载",
  updateDebugUpToDate: "已是最新",
  updateDebugError: "错误",
  updateDebugClear: "恢复真实检查",
  saveSettingsFailed: "保存设置失败",
  backAction: "返回",
  lanReceiverTitle: "LocalSend",
  lanTransferStatusRunning: "已开启",
  lanTransferStatusStopped: "已停止",
  lanTransferStatusPortInUse:
    "端口 53317 被占用，可能本机 LocalSend 正在运行",
  lanTransferStatusActive: "传输中",
  lanTransferStatusDone: "已完成",
  lanTransferStatusFailed: "失败",
  lanTransferStatusCancelled: "已取消",
  lanTransferDevices: "附近设备",
  lanTransferRefresh: "刷新",
  lanScanMenuTitle: "选择网段",
  lanScanMenuHint:
    "只扫描所选网段，其余网段不会被探测。",
  lanScanLastUsed: "上次",
  lanScanCancel: "取消扫描",
  lanScanNoInterface: "没有可用的局域网网卡。",
  lanScanInvalidSubnet: "网段无效，仅支持 /24。",
  lanScanNoSubnet: "请选择要扫描的网段。",
  lanTransferAddDevice: "添加",
  lanTransferIpPlaceholder:
    "输入对端 IP（如 192.168.1.20）",
  lanTransferNoDevices:
    "暂时没有发现设备，请确保对端 LocalSend 已打开接收",
  lanTransferTrusted: "已信任",
  lanTransferSendFile: "发送文件",
  lanTransferSendText: "发送文本",
  lanTransferSendTextTitle: "发送文本给 {name}",
  lanTransferSendTextPlaceholder:
    "输入要发送的文本，对端会直接显示为消息",
  lanTransferCancel: "取消",
  lanTransferWebReceive: "浏览器传给我",
  lanTransferWebReceiveHint:
    "浏览器打开后可上传文件，也可输入文本发送到本机剪贴板。",
  lanTransferWebIdleHint:
    "选择一个模式后生成手机扫码地址：两种模式互斥，切换时会重启服务。",
  lanTransferWebStop: "关闭扫码页",
  lanTransferStartService: "启动服务",
  lanTransferStopService: "停止服务",
  lanTransferDisabledHint:
    "局域网互传已在设置中关闭，可在设置页“互传”分组重新开启。",
  lanIncomingTitle: "收到传输请求",
  lanIncomingAccept: "接受",
  lanIncomingDecline: "拒绝",
  lanIncomingAcceptAndTrust: "接受并信任",
  lanTransferTitle: "LocalSend",
  lanTransferDropFiles: "松开以发送文件",
  windowControlStyle: "窗口控件样式",
  windowControlStyleTip: "仅影响当前应用的标题栏按钮，切换后立即生效。",
  windowControlTrafficLights: "红绿灯",
  windowControlWindows: "Windows",
  minimizeAction: "最小化",
  maximizeAction: "最大化",
  restoreAction: "还原",
  lanTransferSend: "发送",
  openAction: "打开",
  revealInExplorer:
    "在文件管理器中显示",
  lanTransferDownloadDirMissing:
    "互传文件保存目录不存在",
  lanTransferDownloadDirNotDirectory:
    "互传文件保存位置不是目录",
  lanTransferDownloadDirNotWritable:
    "互传文件保存目录不可写",
  lanTransferPin: "接收 PIN",
  lanTransferPinTip:
    "设置后，其他设备必须输入相同 PIN 才能传输；留空表示不需要。",
  lanTransferPinPlaceholder: "留空表示不需要",
  lanPinTitle: "输入 PIN",
  lanPinHint:
    "该设备启用了 PIN，请输入后继续。",
  lanPinPlaceholder: "PIN",
  lanPinInvalid: "PIN 不正确，请重新输入。",
  lanErrorPortInUse:
    "端口 53317 已被占用，本机可能有其它 LocalSend 客户端正在运行。",
  lanErrorMulticastUnavailable:
    "组播不可用，设备发现可能失败；可手动添加设备连接。",
  lanWarningMulticastWindows:
    "组播不可用，设备发现可能失败；请检查 Windows 防火墙是否放行 Power Paste，或手动添加设备连接。",
  lanWarningMulticastMacos:
    "组播不可用，设备发现可能失败；请在「系统设置 → 隐私与安全性 → 本地网络」中允许 Power Paste，或手动添加设备连接。",
  lanWarningMulticastLinux:
    "组播不可用，设备发现可能失败；请检查防火墙（ufw/firewalld）是否放行 TCP 53317 与 UDP 组播 224.0.0.167:53317，或手动添加设备连接。",
  lanErrorListenerFailed:
    "局域网监听失败，已尝试自动恢复；若仍失败请重启服务。",
  lanErrorServiceFailed: "局域网服务启动失败。",
  lanErrorNotRunning: "局域网服务未运行。",
  lanErrorDeviceMissing:
    "找不到该设备，请刷新设备列表后重试。",
  lanErrorNoFiles: "没有可发送的文件。",
  lanErrorFileNotFound: "文件不存在或已被移动。",
  lanErrorRequestMissing: "该请求已失效。",
  lanErrorPinRequired:
    "对端启用了 PIN，请输入 PIN 后重试。",
  lanErrorDeclined: "对方拒绝了本次传输。",
  lanErrorBusy:
    "对方正在处理其它传输，请稍后再试。",
  lanErrorTooManyRequests:
    "PIN 尝试次数过多，对端已暂时拒绝本机；请在对端重新开始接收或重启对端应用后再试。",
  lanErrorEmptyText: "请输入要发送的文本。",
  lanErrorGeneric: "传输失败：{detail}",
  settingsCategorySync: "同步",
  webdavSyncEnabled: "WebDAV 同步",
  webdavAutoSync: "自动同步",
  webdavServerUrl: "WebDAV 地址",
  webdavServerUrlTip:
    "填写 WebDAV 服务根地址，建议使用 HTTPS。",
  webdavUsername: "用户名",
  webdavPassword: "密码",
  webdavPasswordPlaceholder: "留空表示不更改",
  webdavPasswordSaved: "已保存到系统凭据",
  webdavPasswordSavedPlaceholder:
    "密码已保存，输入新密码可覆盖",
  webdavRemoteDir: "远程目录",
  webdavSyncStatus: "同步状态",
  webdavLastSyncAt: "上次同步：{time}",
  webdavNeverSynced: "尚未同步",
  webdavTestConnection: "测试连接",
  webdavSyncNow: "立即同步",
  webdavSyncing: "同步中",
  webdavClearPassword: "清除密码",
  webdavSyncFailed: "WebDAV 同步失败",
  webdavSettingsIncomplete:
    "请先填写 WebDAV 地址和用户名",
  webdavCredentialMissing:
    "请先填写并保存 WebDAV 密码",
  webdavConnectionFailed:
    "WebDAV 连接失败，请检查地址、用户名和密码",
  webdavUnauthorized:
    "WebDAV 认证失败（401），坚果云需使用应用密码，不是登录密码",
  webdavForbidden:
    "WebDAV 权限不足（403），请检查账号权限和远程目录",
  webdavNotFound:
    "WebDAV 地址不存在（404），坚果云通常使用 https://dav.jianguoyun.com/dav/",
  webdavMethodNotAllowed:
    "WebDAV 方法不被该路径支持（405），请检查地址是否为 WebDAV 根地址",
  webdavServiceUnavailable:
    "WebDAV 服务暂时不可用（503），可能是服务端限流或正在处理大量文件，请稍后再试",
  webdavEndpointGone:
    "WebDAV 地址返回 410 Gone，请检查服务地址是否正确。坚果云通常使用 https://dav.jianguoyun.com/dav/",
  webdavRemoteCleanupFailed:
    "内容已上传，但远程删除标记清理失败，请再次同步",
  webdavManifestSaveFailed:
    "内容已上传，但同步索引保存失败，请再次同步",
  webdavManifestFetchFailed:
    "读取 WebDAV 同步索引失败，请再试",
  webdavItemUploadFailed:
    "上传历史条目失败，请检查 WebDAV 空间和网络",
  webdavItemDownloadFailed:
    "下载远程历史条目失败，请再试",
  webdavFolderCreateFailed:
    "创建 WebDAV 同步目录失败，请检查远程目录权限",
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
    "Direct paste on Linux (Wayland) needs wtype, ydotool, or the desktop's remote-input portal (RemoteDesktop). Example installs: `sudo apt install wtype`, `sudo dnf install wtype`, or `sudo pacman -S wtype`. GNOME and KDE compositors do not implement the virtual keyboard protocol (wtype reports “Compositor does not support the virtual keyboard protocol”), so allow remote interaction in the system dialog instead, or configure ydotool (kernel uinput, requires ydotoold).",
  pasteTargetFocusFailed:
    "The target window could not be focused. Paste was cancelled.",
  pasteTargetPermissionDenied:
    "Power Paste does not have the required Accessibility or Automation permission. Allow it in System Settings > Privacy & Security > Accessibility / Automation and try again.",
  pastePortalDenied:
    "The desktop did not allow remote input, so the paste shortcut was rejected. Turn on “Allow Remote Interaction” in the system dialog and confirm, or copy to the clipboard and paste manually.",
  pastePortalUnavailable:
    "This Wayland desktop implements neither the virtual keyboard protocol wtype needs nor the remote-input portal (RemoteDesktop), so the paste shortcut cannot be sent. Configure ydotool (kernel uinput, requires ydotoold), or switch to copy-and-paste-manually in the settings.",
  pastePortalFailed:
    "The desktop remote-input portal failed to send the paste shortcut. Retry, or switch to copy-and-paste-manually in the settings.",
  pasteAuthorizationPending:
    "Automatic paste needs a one-time authorization: in the “Remote Desktop” window, turn on “Allow Remote Interaction” and click “Share”. The grant is reused for the rest of this run.",
  pastePortalTimeout:
    "Timed out waiting for the desktop authorization, so this paste was cancelled. Retry and allow “Remote Interaction” in the “Remote Desktop” window.",
  unsupportedLaunchOnStartup:
    "Launch on startup is not available on this platform.",
  duplicateShortcut: "Shortcuts must be unique.",
  invalidShortcut: "The shortcut format is invalid.",
  shortcutRegistrationFailed:
    "The shortcut is not active. It may already be used by another app.",
  shortcutConflictMessage:
    "{name} ({shortcut}) is not active. It may already be used by another app. Close the conflicting app and retry, or choose a different shortcut in Settings.",
  waylandPortalUnavailable:
    "This Wayland desktop does not provide the GlobalShortcuts portal, so shortcuts cannot be bound automatically. Assign a shortcut to Power Paste in your system settings, or use an X11 session.",
  waylandPortalDenied:
    "The desktop did not bind this shortcut: the confirmation dialog may have been dismissed, or the shortcut conflicts with a system shortcut. Retry and confirm in the dialog, or choose another key.",
  waylandPortalFailed:
    "The desktop failed to bind the global shortcut. Retry; if it keeps failing, assign a shortcut to Power Paste in your system settings.",
  waylandPortalClosed:
    "The desktop released the shortcut binding (for example after a session restart). Retry to bind it again.",
  waylandPortalNoAppId:
    "The desktop could not identify the app id of this process, so it refused to bind shortcuts. Launch Power Paste from your app list (run pnpm desktop:install first for a local build), or start it with pnpm tauri dev from the project directory.",
  waylandPortalInvalidAppId:
    "The desktop rejected this process's app id “{appId}”: GNOME only accepts reverse-DNS application ids (at least one dot, for example com.yulei.powerpaste), so the bind request was discarded. Launch Power Paste from your app list (run pnpm desktop:install first for a local build), or start it with pnpm tauri dev from the project directory.",
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
  lanTransferCancel: "Cancel",
  lanTransferWebReceive: "Upload from a browser",
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
  lanTransferConversations: "设备会话",
  lanLocalDevice: "本机",
  lanPeerOnline: "在线",
  lanPeerOffline: "离线",
  lanTransferQrTitle: "扫码互传",
  lanConversationSelectHint: "选择左侧设备开始传输",
  lanConversationEmpty: "粘贴文字或拖入文件，回车即可发送",
  lanComposerPlaceholder: "输入消息，回车发送，Shift + 回车换行",
  lanComposerAttach: "添加附件",
  lanComposerAttachments: "待发送附件",
  lanDropToPeer: "松开即可发送到 {name}",
  lanDropSelectPeer: "把文件拖到左侧设备上即可发送",
  lanPeerOfflineRetry: "设备不在线，已重试一次仍未连上，请刷新设备列表",
  lanResend: "重新发送",
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
  lanErrorResendUnavailable: "这条记录无法重新发送：发送内容已不在内存中。",
  lanTransferReceiveLink: "通过链接接收",
  lanTransferDeviceName: "设备名称",
  lanTransferLocalIp: "本机 IP",
  lanTransferUnknownIp: "暂未获取到 IP",
  lanTransferPort: "端口",
  lanTransferUnknownDevice: "未知设备",
  lanTransferUnknownModel: "未知型号",
  lanTransferManualSend: "手动发送",
  lanTransferSendHelp: "请确保接收设备连接到同一局域网。",
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
  lanTransferConversations: "Device conversations",
  lanLocalDevice: "This device",
  lanPeerOnline: "Online",
  lanPeerOffline: "Offline",
  lanTransferQrTitle: "Scan to transfer",
  lanConversationSelectHint: "Pick a device on the left to start",
  lanConversationEmpty: "Paste text or drop files, then press Enter to send",
  lanComposerPlaceholder: "Type a message. Enter to send, Shift + Enter for a new line",
  lanComposerAttach: "Add attachment",
  lanComposerAttachments: "Attachments to send",
  lanDropToPeer: "Release to send to {name}",
  lanDropSelectPeer: "Drop the files onto a device on the left to send",
  lanPeerOfflineRetry:
    "The device is offline. Reconnecting was attempted once but failed; refresh the device list.",
  lanResend: "Send again",
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
  lanErrorResendUnavailable:
    "This record cannot be sent again: its content is no longer kept in memory.",
  lanTransferReceiveLink: "Receive via link",
  lanTransferDeviceName: "Device name",
  lanTransferLocalIp: "Local IP",
  lanTransferUnknownIp: "No IP address found",
  lanTransferPort: "Port",
  lanTransferUnknownDevice: "Unknown device",
  lanTransferUnknownModel: "Unknown model",
  lanTransferManualSend: "Manual sending",
  lanTransferSendHelp: "Make sure the receiving device is on the same LAN.",
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
