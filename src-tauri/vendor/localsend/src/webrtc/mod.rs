// signaling 仅在启用 webrtc-signaling feature 时才会被使用；未启用时整块都是死代码。
// 这里放宽该模块的 lint，保持与上游源码一致的同时避免构建噪声。
#[allow(dead_code, unused_imports)]
pub mod signaling;
#[cfg(feature = "webrtc")]
pub mod webrtc;
