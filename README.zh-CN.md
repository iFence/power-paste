# Power Paste

Power Paste 是一个基于 `Tauri 2`、`Vue 3` 和 `Rust` 构建的桌面剪贴板历史管理器。

## 平台状态

- Windows：已实现完整能力，包括剪贴板监听、写回系统剪贴板、直接粘贴到上一个目标应用、开机启动、托盘和全局快捷键
- macOS：目标是可启动、可打包；Windows 原生能力当前会显示为暂不支持
- Linux：目标是可启动、可打包；Windows 原生能力当前会显示为暂不支持

## 当前功能

- 通过全局快捷键打开剪贴板历史面板
- 捕获文本、图片和图文混合内容
- 按类型搜索和筛选历史记录
- 置顶重要条目
- 编辑纯文本历史条目
- 记录来源应用信息
- 主题、强调色、语言和列表密度设置
- 敏感应用忽略列表
- 托盘集成与单实例运行
- 本地持久化保存历史、设置和图片资源

## 跨平台降级策略

以下能力当前仅在 Windows 上可用，在 macOS 和 Linux 上会显示为友好的“暂不支持”提示：

- 将条目内容写回系统剪贴板
- 直接粘贴到上一个目标应用
- 开机启动
- 原生图文混合内容回放

与平台无关的功能，例如历史浏览、搜索、筛选、置顶、编辑、删除和设置保存，仍然保持可用。

## 技术栈

- `Tauri 2`
- `Vue 3`
- `Vite`
- `Rust`
- Windows 原生剪贴板集成通过 `PowerShell`、Win32 API 和 `System.Windows.Forms` 实现

## 环境要求

- Node.js 18+
- `pnpm` 10+
- Rust 1.77+

Windows 开发环境还需要：

- Windows 10 或 Windows 11
- WebView2 Runtime

## 开发

安装依赖：

```bash
pnpm install
```

仅运行前端：

```bash
pnpm dev
```

运行桌面应用：

```bash
pnpm tauri dev
```

## 构建

前端构建：

```bash
pnpm build
```

Rust 检查：

```bash
cd src-tauri
cargo check
```

桌面包构建：

```bash
pnpm tauri build
```

## 数据存储

Power Paste 会将本地数据保存到 Tauri 的 app-local-data 目录中。
常见文件包括：

- `history.json`
- `settings.json`
- `images/`

## 项目结构

```text
.
- src/                 # Vue 界面
- src/components/      # UI 组件
- src/composables/     # 前端状态和行为封装
- src/services/        # Tauri API 封装
- src/utils/           # 前端工具函数
- src/styles/          # 样式
- src-tauri/src/       # Rust 后端
```
