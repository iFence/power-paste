# LocalSend 核心库（内嵌副本）

本目录是 [localsend/localsend](https://github.com/localsend/localsend) 仓库中 `packages/core`
的源码快照，用于在 power-paste 内直接实现 LocalSend 协议（Apache License 2.0，见本目录 `LICENSE`）。

- 上游版本：`v1.18.2`（tag 快照）
- 快照来源：`https://github.com/localsend/localsend` 的 `packages/core/`
- 引入方式：`src-tauri/Cargo.toml` 中的 path 依赖，crate 名 `localsend`
- 启用 feature：`crypto`、`discovery`、`http`、`multicast`（不启用 `webrtc`/`webrtc-signaling`）
- 本地修改：仅一处 lint 属性（见下），其余文件与上游 tag 快照逐字一致，便于后续按版本对比升级。

### 本地补丁

`src/webrtc/mod.rs` 中的 `signaling` 模块加了一行 `#[allow(dead_code, unused_imports)]`：
该模块只在 `webrtc-signaling` feature 下才会被使用，而 power-paste 不启用 WebRTC，
不加这行会在每次构建时输出 8 条无效警告。保留模块本身是为了不丢失上游的单元测试。

## 为什么内嵌而不是依赖 crates.io

上游没有把该 crate 发布到 crates.io（同名 crate 属于第三方项目），且仓库内的 API 仍在演进，
因此以源码快照的方式引入，既能离线构建，也能在需要时以最小补丁适配。

## 升级步骤

1. 取上游新 tag 的 `packages/core` 目录。
2. 覆盖本目录（保留本说明文件）。
3. 运行 `cargo test -p localsend --features crypto,discovery,http,multicast -- --skip formats_nanosecond_timestamp`
   与 `cargo test -p power-paste`，并手动验证与官方客户端互传。
4. 重新应用上面的 lint 补丁，否则构建会再次出现 8 条警告。

注意：`rust-toolchain.toml` 中上游固定使用 Rust 1.97.1，本仓库的 `rust-version` 与之保持一致。

已知上游问题：`model::transfer::tests::formats_nanosecond_timestamp` 断言 1ns 精度，
而 Windows 的 `SystemTime` 只有 100ns 精度，因此该用例在 Windows 上必然失败，上面用 `--skip` 跳过。
