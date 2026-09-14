# Third-party notices

Power Paste 包含以下第三方开源组件。完整许可证文本随对应目录提供。

## LocalSend 核心库（Rust）

- 项目：LocalSend（https://github.com/localsend/localsend）
- 组件：`packages/core`（crate 名 `localsend`）
- 版本：v1.18.2 源码快照
- 许可证：Apache License 2.0（见 `src-tauri/vendor/localsend/LICENSE`）
- 位置：`src-tauri/vendor/localsend/`
- 本地修改：仅一处 lint 属性——`src/webrtc/mod.rs` 的 `signaling` 模块加了
  `#[allow(dead_code, unused_imports)]`，用于消除未启用 WebRTC 时的构建警告；其余源码未改动

Power Paste 通过该库实现 LocalSend 协议，使安装了 power-paste 的设备可以与官方 LocalSend
客户端互相发现与传输文件；power-paste 自身的界面、剪贴板历史与设置项均为本项目实现。

## LocalSend 应用图标

- 项目：LocalSend（https://github.com/localsend/localsend）
- 资源：官方应用图标（`org.localsend.localsend_app`）
- 位置：`public/localsend.png`
- 版权归 LocalSend 项目所有，本项目只用于在历史记录中标识「内容来自 LocalSend 传输」
