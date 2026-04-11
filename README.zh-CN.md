# Power Paste

Power Paste 是一个基于 `Tauri 2`、`Vue 3` 和 `Rust` 构建的桌面剪贴板历史管理器。它围绕“原生感”的桌面工作流设计：后台监听剪贴板变化，通过全局快捷键呼出紧凑面板，然后快速搜索、预览、复制、编辑或直接把历史内容粘贴回上一个目标应用。

它不只是一个“能用”的剪贴板工具，也是一款强调界面质感的桌面应用。Power Paste 希望把高频剪贴板操作做得足够顺手，同时把透明面板、浅深色主题、主题色切换和细腻的视觉层次打磨到日常常开也依然舒服。

English version: [README.md](./README.md)。

## 产品预览

| 主面板（浅色） | 主面板（深色） |  设置 |
|---|---|---|
| ![Power Paste light theme](./docs/light.png) | ![Power Paste dark theme](./docs/dark.png) |![Power Paste settings panel](./docs/settings.png)|

## 为什么是 Power Paste

- 高效：全局快捷键一键呼出，复制过的内容随手可搜、可筛、可回放
- 原生感：围绕桌面工作流设计，不打断当前输入节奏
- 高颜值：透明浮层、浅深色主题、主题色切换，让工具本身也具备观感价值
- 可常驻：托盘、单实例、启动检查更新等能力让它适合长期驻留在系统中

## 核心特性

- 通过全局快捷键呼出历史面板
- 捕获文本、图片和图文混合剪贴板内容
- 自动识别复制的链接，并可从历史条目中直接用默认浏览器打开
- 支持 `全部`、`置顶`、`文本`、`图片`、`图文` 筛选
- 重要条目可置顶
- 纯文本历史条目可直接编辑
- 支持将历史内容重新写回系统剪贴板，或直接粘贴回上一个目标应用
- 鼠标悬停图片缩略图可预览大图
- 支持语言、主题、主题色、开机启动、历史数量、图片大小、全局快捷键、调试模式等设置
- 支持系统托盘、单实例运行、启动后自动检查更新，以及从托盘菜单手动检查更新
- 使用 SQLite 持久化历史记录，并在磁盘上保存图片资源

## 平台状态

- Windows：当前主目标平台，已实现剪贴板监听、写回剪贴板、直接粘贴、开机启动、托盘、更新检查和全局快捷键等完整流程
- macOS：已支持剪贴板监听、写回系统剪贴板和直接粘贴，但直接粘贴依赖系统授予“辅助功能 / 自动化”权限
- Linux：目标是可启动、可打包；Windows 特有的原生剪贴板工作流当前会降级为友好的“暂不支持”提示

## 功能说明

### 历史面板交互

- 主界面以紧凑的透明浮层窗口形式展示
- 可使用上下方向键在当前筛选结果中移动选中项，并自动滚动到可视区域
- `Enter` 可在支持的平台上将当前选中项直接粘贴回上一个目标应用
- `Ctrl/Cmd + C` 可将当前选中历史项重新复制到系统剪贴板
- 双击历史项时，如平台支持直接粘贴，会立即执行粘贴
- 链接条目会在右下角显示打开按钮，可直接调用系统默认浏览器

### 条目类型

- 文本条目：支持搜索、编辑、复制、直接粘贴
- 链接条目：从复制的 URL 自动识别，可直接调用系统默认浏览器打开
- 图片条目：支持缩略图、大图悬停预览、复制与粘贴
- 图文条目：在后端支持的情况下保留混合内容并回放

### 设置项

- 界面语言：简体中文 / English
- 界面主题：浅色 / 深色 / 跟随系统
- 主题颜色：海蓝 / 琥珀 / 青玉 / 玫瑰
- 开机启动
- 最大历史数量
- 最大图片大小
- 全局快捷键录制与清空
- 调试模式开关

更新检查不再放在设置页中配置。应用会在启动时自动检查更新；如果检测到新版本，顶部会显示更新图标；同时托盘菜单也提供“检查更新”入口。

### 原生桌面集成

- 单实例运行：重复启动时复用现有窗口实例
- 托盘支持：应用可常驻后台，并提供“主面板 / 检查更新 / 退出”菜单项
- 通过 Tauri 插件注册全局快捷键
- 通过 Tauri Updater 插件执行更新检查

## 跨平台降级策略

以下能力当前仍以 Windows 为优先，Linux 上可能显示“暂不支持”；macOS 的直接粘贴依赖系统权限：

- 开机启动
- 原生图文混合内容回放

历史浏览、搜索、筛选、置顶、编辑、删除、设置保存等通用界面能力仍然可用。

## 技术栈

### 前端

- `Vue 3`
- `Vite`
- 基于 Composition API 的 Composable 状态与交互组织方式

### 桌面端 / 后端

- `Tauri 2`
- `Rust`
- `tauri-plugin-global-shortcut`
- `tauri-plugin-autostart`
- `tauri-plugin-single-instance`
- `tauri-plugin-updater`
- `tauri-plugin-sql` + SQLite
- `tauri-plugin-clipboard-next`

### Windows 原生能力

- Win32 API
- WebView2
- 基于 PowerShell 的 Windows 剪贴板与直接粘贴辅助流程

## 环境要求

- Node.js `18+`
- `pnpm` `10+`
- Rust `1.77.2+`

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

## 数据存储

应用数据会保存在 Tauri 的 app-local-data 目录中。

当前常见持久化内容包括：

- SQLite 历史数据库
- `settings.json`
- 磁盘上的图片缓存文件

当前实现中，历史记录主存储已经不是简单的 `history.json` 文件，而是基于 SQLite。

## 项目结构

```text
.
├── src/
│   ├── components/      # 可复用 Vue 界面组件
│   ├── composables/     # 前端状态与交互逻辑
│   ├── services/        # Tauri invoke / event 封装
│   ├── styles/          # 全局样式
│   └── utils/           # 前端工具函数
├── src-tauri/
│   ├── src/commands.rs   # Tauri 命令入口
│   ├── src/runtime.rs    # 窗口与运行时行为
│   ├── src/update.rs     # 应用更新流程
│   ├── src/repository.rs # SQLite 历史存储
│   ├── src/storage.rs    # 设置与路径存储
│   └── src/clipboard/    # 剪贴板后端与平台能力封装
└── scripts/              # 本地开发辅助脚本
```

## 仓库说明

- 包管理器：`pnpm`
- 前端主要语言：JavaScript / Vue SFC
- 桌面后端语言：Rust
- 当前工作区默认分支：`master`

## License

本项目采用 GNU Affero General Public License v3.0（AGPLv3）许可证。

完整许可证文本请见 [LICENSE](./LICENSE)。

如果你修改了本项目并通过网络向用户提供服务，AGPLv3 要求你向这些用户提供对应修改版本的完整源代码。
