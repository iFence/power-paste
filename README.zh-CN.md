# Power Paste

Power Paste 是一个基于 `Tauri 2`、`Vue 3` 和 `Rust` 构建的桌面剪贴板历史管理器。它围绕“原生感”的桌面工作流设计：后台监听剪贴板变化，通过全局快捷键呼出紧凑面板，然后快速搜索、预览、复制、编辑、打标签，或把历史内容直接粘贴回上一个目标应用。

当前实现是本地优先架构。剪贴板历史保存在本机 SQLite 中，设置保存到 `settings.json`，手机互传则由桌面应用临时开启局域网服务并通过二维码让手机浏览器接入。

English version: [README.md](./README.md)。

## 产品预览

| 主面板（浅色） | 手机互传（深色） | 设置 |
|---|---|---|
| ![Power Paste light theme](./docs/light.png) | ![Power Paste dark theme](./docs/qr.png) | ![Power Paste settings panel](./docs/settings.png) |

## 核心亮点

- 全局快捷键呼出主历史面板
- 支持捕获 `文本`、`链接`、`图片`、`图文混合` 剪贴板内容
- 支持 `全部`、`置顶`、`文本`、`图片`、`图文` 筛选
- 重要条目可置顶，并且不会被批量清空或保留策略自动清理
- 支持收藏条目，提供额外视觉优先级标识
- 支持 Finder 风格颜色标签：每条最多 `3` 个标签，固定 `7` 种颜色，可自定义标签名称
- 纯文本历史条目支持原地编辑
- 可重新写回系统剪贴板，或在平台支持时直接粘贴回上一个目标应用
- 支持通过二维码在手机浏览器和电脑之间互传文本、图片和文件
- 支持语言、主题、主题色、密度、开机启动、声音、历史保留策略、图片大小、互传目录、标签名称、调试模式、全局快捷键等设置
- 支持托盘、单实例、启动自动检查更新和手动检查更新
- 清空历史、恢复设置等危险操作使用应用内自定义确认弹窗，而不是系统弹窗

## 当前功能说明

### 历史面板交互

- 主界面是紧凑的透明浮层窗口
- 当前搜索和筛选条件下的数量统计会准确更新
- `Enter` 可在支持的平台上把当前选中项直接粘贴回目标应用
- `Ctrl/Cmd + C` 可将当前选中项重新复制到系统剪贴板
- 双击条目时，如平台支持直接粘贴，会立即执行粘贴
- 链接条目可直接调用系统默认浏览器打开
- `清空历史` 只会删除未置顶条目

### 条目类型

- `文本`：支持搜索、编辑、复制，并在支持的平台上直接粘贴
- `链接`：从复制的 URL 自动识别，可调用系统默认浏览器打开
- `图片`：支持缩略图、大图悬停预览，以及在支持的平台上进行回放
- `图文混合`：在后端能力允许时保留组合载荷并尽量回放

### 标签与整理能力

- 每条历史记录最多支持 `3` 个标签
- 内置标签颜色采用 Finder 风格调色板：`红`、`橙`、`黄`、`绿`、`蓝`、`紫`、`灰`
- 标签颜色和显示名称分离
- 标签名称可在设置页中修改
- 主面板支持按标签直接聚合筛选

### 手机电脑互传

- 从桌面端启动临时局域网互传会话
- 手机扫码后用浏览器直接打开互传页面
- 不需要安装手机 App
- 支持桌面端向手机发送文本和文件
- 支持手机向桌面发送文本、图片和文件
- 桌面端收到的文件会保存到设置中的下载目录
- 桌面端互传历史可直接打开或定位接收文件
- 会显示连接 / 断开状态，并在空闲超时后自动清理会话

### 设置页

当前设置页按以下分类组织：

- `通用`
- `历史`
- `互传`
- `快捷键`
- `高级`
- `关于`

当前可配置项包括：

- 语言：简体中文 / English
- 主题模式：浅色 / 深色 / 跟随系统
- 主题色：海蓝 / 琥珀 / 青玉 / 玫瑰
- 界面密度：紧凑 / 宽松
- 开机启动
- 复制提示音
- 最大历史数量
- 未置顶历史的最大保留天数
- 最大图片存储大小
- 标签显示名称
- 局域网互传下载目录
- 全局快捷键录制与清空
- 调试模式

更新检查不作为普通设置项配置。应用会在启动时自动检查更新；检测到新版本时，界面会显示更新徽标；托盘菜单也提供手动检查更新入口。

## 平台状态

- Windows：当前主目标平台，也是目前混合剪贴板回放和目标感知直接粘贴能力最完整的平台
- macOS：直接粘贴依赖系统授予“辅助功能 / 自动化”权限
- Linux：直接粘贴依赖 `X11 + xdotool` 或 `Wayland + wtype`；缺少对应工具时，界面会直接提示当前会话所需的安装依赖，而不是只显示泛化的不支持提示；图文混合回放仍会退化为单一优先载荷

### Linux 说明

- `xdotool` 和 `wtype` 都不是 Linux 平台的必备基础依赖，只在“把历史内容直接粘贴回上一个目标应用”时需要。
- 如果当前是 `X11` 会话，需要安装 `xdotool` 才能启用直接粘贴。
- 如果当前是 `Wayland` 会话，需要安装 `wtype` 才能启用直接粘贴。
- 当缺少对应工具时，Power Paste 仍可正常执行“复制回系统剪贴板”，并会根据当前会话类型给出明确安装提示。

### macOS 升级后重新授权

如果从旧版本升级后，直接粘贴仍提示缺少“辅助功能”或“自动化”权限，可能是 macOS 仍将权限记录绑定到旧的应用构建。可以按以下步骤重新授权：

1. 退出 Power Paste。
2. 执行：

```bash
xattr -dr com.apple.quarantine /Applications/Power\ Paste.app
```

3. 打开“系统设置 > 隐私与安全性 > 辅助功能”，将 Power Paste 关闭后重新打开。
4. 打开“系统设置 > 隐私与安全性 > 自动化”，如果列表中出现 Power Paste，也重新启用。
5. 重新启动 Power Paste 后再尝试直接粘贴。

## 架构说明

- 历史记录以 SQLite 作为唯一事实来源
- 前端状态采用事件驱动更新，但不替代 SQLite 成为主存储
- 设置持久化到 `settings.json`
- 局域网互传接收的文件存入设置中指定的下载目录
- 主面板窗口尺寸与设置页窗口尺寸分开持久化
- 当前代码仓库尚未实现 WebDAV 历史同步

## 技术栈

### 前端

- `Vue 3`
- `Vue Router`
- `Vite`
- 基于 Composition API 的 Composable 组织方式

### 桌面端 / 后端

- `Tauri 2`
- `Rust`
- `tauri-plugin-global-shortcut`
- `tauri-plugin-autostart`
- `tauri-plugin-single-instance`
- `tauri-plugin-updater`
- `tauri-plugin-sql` + SQLite
- `tauri-plugin-clipboard-next`
- `tauri-plugin-dialog`
- `tiny_http`，用于临时手机互传服务

### 平台集成

- Windows：Win32 API、WebView2、PowerShell 辅助流程
- macOS：AppKit / Objective-C 原生集成
- Linux：依赖桌面自动化工具完成直接粘贴降级方案

## 环境要求

- Node.js `18+`
- `pnpm` `10+`
- Rust `1.77.2+`

Linux 如果需要直接粘贴，还需要以下其一：

- X11 会话 + `xdotool`
- Wayland 会话 + `wtype`

常见安装示例：

```bash
# Ubuntu / Debian
sudo apt install xdotool
sudo apt install wtype

# Fedora
sudo dnf install xdotool
sudo dnf install wtype

# Arch Linux
sudo pacman -S xdotool
sudo pacman -S wtype
```

Windows 开发环境还需要：

- Windows 10 或 Windows 11
- Microsoft WebView2 Runtime

## 开发

安装依赖：

```bash
pnpm install
```

仅运行前端：

```bash
pnpm dev
```

运行 Tauri 桌面应用：

```bash
pnpm tauri dev
```

## 构建

构建前端：

```bash
pnpm build
```

执行 Rust 检查：

```bash
cd src-tauri
cargo check
```

构建桌面安装包：

```bash
pnpm tauri build
```

## 持续校验

仓库当前包含两个 GitHub Actions 工作流：

- `verify.yml`：在日常 `push / pull request` 时执行，检查 `Windows / Linux / macOS` 三个平台上的前端构建、Rust 测试和 release 编译
- `release.yml`：在版本 Tag 上执行，负责真实的跨平台 Tauri 发版打包

## 数据存储

应用数据保存在 Tauri 的 app-local-data 目录中。

当前常见持久化内容包括：

- SQLite 历史数据库
- 文本、富文本、图片载荷及标签元数据
- 在保留原始字节时保存的上传图片原始内容
- 局域网互传接收到的文件
- `settings.json`

当前实现中，历史记录主存储已经不是简单的 `history.json` 文件。

## 项目结构

```text
.
├── src/
│   ├── components/      # 可复用 Vue UI 组件
│   ├── composables/     # 前端状态与交互逻辑
│   ├── router/          # 路由声明
│   ├── services/        # Tauri invoke / event 封装
│   ├── styles/          # 全局样式
│   ├── utils/           # 前端常量与工具函数
│   └── views/           # 页面级视图
├── src-tauri/
│   ├── src/commands/    # 按领域拆分的 Tauri 命令入口
│   ├── src/clipboard/   # 剪贴板捕获与回放后端
│   ├── src/lan_receiver.rs
│   ├── src/repository.rs
│   ├── src/runtime.rs
│   ├── src/storage.rs
│   ├── src/update.rs
│   └── src/usecases.rs
└── scripts/             # 本地开发辅助脚本
```

## 支持项目

如果 Power Paste 对你有帮助，欢迎支持项目开发：

| 支付宝 | 微信支付 | 赞赏码 |
|---|---|---|
| ![支付宝](./docs/alipay.jpg) | ![微信支付](./docs/wechatpay.jpg) |![赞赏码](./docs/appreciation.jpg)|

你的支持将帮助维护和改进 Power Paste。感谢！

## License

本项目采用 GNU Affero General Public License v3.0（AGPLv3）许可证。

完整许可证文本请见 [LICENSE](./LICENSE)。

如果你修改了本项目并通过网络向用户提供服务，AGPLv3 要求你向这些用户提供对应修改版本的完整源代码。
