//! GNOME / KDE Wayland 下的按键注入后端：XDG Desktop Portal 的 RemoteDesktop 接口。
//!
//! mutter（GNOME）与 KWin 出于安全考虑都不实现 `zwp_virtual_keyboard_manager_v1`，
//! 所以 wtype 在这两个桌面上必然报 “Compositor does not support the virtual
//! keyboard protocol”。桌面留给应用的注入通道是
//! `org.freedesktop.portal.RemoteDesktop`：首次使用时弹出系统授权窗口，用户打开
//! “Remote Interaction” 并确认后，应用即可通过 `NotifyKeyboardKeysym` 发送按键。
//!
//! 授权按会话生效，因此会话在进程内复用：授权一次之后不再弹窗，直到应用退出或
// 会话失效（桌面重启、授权被撤销）。

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        LazyLock, Mutex,
    },
};

use anyhow::{Context, Result};
use zbus::{
    blocking::{Connection, MessageIterator, Proxy},
    message::Type as MessageType,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
    MatchRule,
};

const PORTAL_BUS_NAME: &str = "org.freedesktop.portal.Desktop";
const PORTAL_OBJECT_PATH: &str = "/org/freedesktop/portal/desktop";
const REMOTE_DESKTOP_INTERFACE: &str = "org.freedesktop.portal.RemoteDesktop";
const REQUEST_INTERFACE: &str = "org.freedesktop.portal.Request";

// org.freedesktop.portal.Request::Response 的状态码。
const RESPONSE_SUCCESS: u32 = 0;
const RESPONSE_CANCELLED: u32 = 1;

/// `SelectDevices` 的设备类型位掩码：只申请键盘，不额外索取指针 / 触屏授权。
const DEVICE_KEYBOARD: u32 = 1;

// NotifyKeyboardKeysym 的按键状态与 X11 keysym。
const KEY_STATE_RELEASED: u32 = 0;
const KEY_STATE_PRESSED: u32 = 1;
const KEYSYM_CONTROL_LEFT: i32 = 0xffe3;
const KEYSYM_V: i32 = 0x76;

/// 门户拒绝注入时前端要展示的原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PastePortalError {
    /// 桌面没有 RemoteDesktop 门户（例如精简的 wlroots 会话）。
    Unavailable(String),
    /// 用户在授权窗口里取消，或者之前拒绝并记住了选择。
    Denied,
    /// 授权窗口一直没有被确认（用户没有看到或忽略了它）。
    Timeout,
    /// 门户返回了其它错误。
    Failed(String),
}

impl PastePortalError {
    fn failed(error: impl std::fmt::Display) -> Self {
        Self::Failed(error.to_string())
    }

    /// 前端据此显示本地化提示。
    pub(crate) fn message_key(&self) -> &'static str {
        match self {
            Self::Unavailable(_) => "paste_portal_unavailable",
            Self::Denied => "paste_portal_denied",
            Self::Timeout => "paste_portal_timeout",
            Self::Failed(_) => "paste_portal_failed",
        }
    }

    /// 写进日志的细节，便于排查（不展示给用户）。
    pub(crate) fn detail(&self) -> &str {
        match self {
            Self::Unavailable(detail) | Self::Failed(detail) => detail,
            Self::Denied => "denied by the user",
            Self::Timeout => "authorization dialog was not confirmed",
        }
    }
}

/// 桌面授权窗口需要用户操作，等待太久就直接放弃，避免粘贴命令永远卡住。
const AUTHORIZATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);

/// 是否已经拿到（并复用）远程输入授权。
pub(crate) fn session_ready() -> bool {
    SESSION.lock().unwrap().is_some()
}

/// 门户方法返回的结果字典（`a{sv}`）。
type PortalResults = HashMap<String, OwnedValue>;
/// 门户方法接收的选项字典（`a{sv}`），键固定为静态字符串。
type PortalOptions = HashMap<&'static str, Value<'static>>;

/// 已授权的远程桌面会话。
#[derive(Clone)]
struct PortalSession {
    connection: Connection,
    path: OwnedObjectPath,
}

/// 进程内单例：授权一次后复用同一会话。
static SESSION: LazyLock<Mutex<Option<PortalSession>>> = LazyLock::new(|| Mutex::new(None));

/// 通过桌面门户发送一次 Ctrl+V。
///
/// 首次调用会创建 RemoteDesktop 会话，桌面会弹出授权窗口；授权成功后按键立即
/// 发送到当前焦点窗口。
pub(crate) fn inject_paste_shortcut() -> std::result::Result<(), PastePortalError> {
    let mut guard = SESSION.lock().unwrap();
    if guard.is_none() {
        *guard = Some(open_session()?);
    }

    let session = guard.as_ref().expect("session just created");
    let result = send_paste_keys(session);
    if result.is_err() {
        // 会话可能已失效（桌面重启、授权被撤销），下一次重新申请授权。
        *guard = None;
    }
    result
}

/// 新建并授权一个只申请键盘的远程桌面会话。
fn open_session() -> std::result::Result<PortalSession, PastePortalError> {
    let connection = Connection::session().map_err(classify_connection_error)?;
    let path = create_session(&connection)?;
    select_keyboard_device(&connection, &path)?;
    start_session(&connection, &path)?;
    eprintln!("[paste] 桌面门户已授权键盘注入：{}", path.as_str());

    // 授权窗口刚关闭，焦点还在回到目标窗口的路上，稍等一下再注入按键。
    std::thread::sleep(std::time::Duration::from_millis(300));

    Ok(PortalSession { connection, path })
}

/// 依次发送 Ctrl 按下、V 按下、V 松开、Ctrl 松开。
fn send_paste_keys(session: &PortalSession) -> std::result::Result<(), PastePortalError> {
    notify_keysym(session, KEYSYM_CONTROL_LEFT, KEY_STATE_PRESSED)?;
    notify_keysym(session, KEYSYM_V, KEY_STATE_PRESSED)?;
    notify_keysym(session, KEYSYM_V, KEY_STATE_RELEASED)?;
    notify_keysym(session, KEYSYM_CONTROL_LEFT, KEY_STATE_RELEASED)?;
    Ok(())
}

/// `CreateSession`：拿到会话句柄。
fn create_session(connection: &Connection) -> std::result::Result<OwnedObjectPath, PastePortalError> {
    let handle_token = next_handle_token();
    let options = PortalOptions::from([
        ("handle_token", Value::from(handle_token.clone())),
        ("session_handle_token", Value::from(next_handle_token())),
    ]);

    let results = call_request(connection, "CreateSession", &(options,), &handle_token)?;
    let handle = results
        .get("session_handle")
        .and_then(|value| value_to_string(value))
        .ok_or_else(|| PastePortalError::failed("portal response missing session handle"))?;
    OwnedObjectPath::try_from(handle)
        .map_err(|error| PastePortalError::failed(format!("invalid session handle: {error}")))
}

/// `SelectDevices`：声明只要键盘。
fn select_keyboard_device(
    connection: &Connection,
    session_path: &OwnedObjectPath,
) -> std::result::Result<(), PastePortalError> {
    let handle_token = next_handle_token();
    let options = PortalOptions::from([
        ("handle_token", Value::from(handle_token.clone())),
        ("types", Value::from(DEVICE_KEYBOARD)),
    ]);

    call_request(
        connection,
        "SelectDevices",
        &(session_path.clone(), options),
        &handle_token,
    )
    .map(|_| ())
}

/// `Start`：桌面在这里弹出授权窗口，用户确认后才算授权成功。
fn start_session(
    connection: &Connection,
    session_path: &OwnedObjectPath,
) -> std::result::Result<(), PastePortalError> {
    let handle_token = next_handle_token();
    let options = PortalOptions::from([("handle_token", Value::from(handle_token.clone()))]);

    call_request(
        connection,
        "Start",
        &(session_path.clone(), String::new(), options),
        &handle_token,
    )
    .map(|_| ())
}

/// `NotifyKeyboardKeysym`：实际的按键注入。
fn notify_keysym(
    session: &PortalSession,
    keysym: i32,
    state: u32,
) -> std::result::Result<(), PastePortalError> {
    let proxy = portal_proxy(
        &session.connection,
        PORTAL_OBJECT_PATH,
        REMOTE_DESKTOP_INTERFACE,
    )
    .map_err(PastePortalError::failed)?;

    proxy
        .call_method(
            "NotifyKeyboardKeysym",
            &(
                session.path.clone(),
                PortalOptions::new(),
                keysym,
                state,
            ),
        )
        .map_err(classify_bus_error)?;
    Ok(())
}

/// 调用门户方法并等待对应的 `Request::Response` 信号。
fn call_request<B>(
    connection: &Connection,
    method: &str,
    body: &B,
    handle_token: &str,
) -> std::result::Result<PortalResults, PastePortalError>
where
    B: serde::Serialize + zbus::zvariant::DynamicType,
{
    let request_path = portal_handle_path(connection, "request", handle_token)
        .map_err(PastePortalError::failed)?;
    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface(REQUEST_INTERFACE)
        .map_err(classify_bus_error)?
        .member("Response")
        .map_err(classify_bus_error)?
        .path(request_path.as_str())
        .map_err(classify_bus_error)?
        .build();
    // 先订阅再调用，避免门户响应在订阅前到达导致永久等待。
    let mut responses = MessageIterator::for_match_rule(rule, connection, Some(1))
        .map_err(classify_bus_error)?;

    let proxy = portal_proxy(connection, PORTAL_OBJECT_PATH, REMOTE_DESKTOP_INTERFACE)
        .map_err(PastePortalError::failed)?;
    proxy
        .call_method(method, body)
        .map_err(classify_bus_error)?;

    // 授权窗口由用户操作，等待必须在超时后收敛，不能让粘贴命令永远卡住。
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("wayland-paste-authorization".into())
        .spawn(move || {
            let _ = sender.send(responses.next());
        })
        .map_err(PastePortalError::failed)?;

    let message = match receiver.recv_timeout(AUTHORIZATION_TIMEOUT) {
        Ok(Some(Ok(message))) => message,
        Ok(Some(Err(error))) => return Err(classify_bus_error(error)),
        Ok(None) => return Err(PastePortalError::failed("portal response stream closed")),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => return Err(PastePortalError::Timeout),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            return Err(PastePortalError::failed("portal response channel closed"))
        }
    };
    let (code, results) = message
        .body()
        .deserialize::<(u32, PortalResults)>()
        .map_err(PastePortalError::failed)?;

    match code {
        RESPONSE_SUCCESS => Ok(results),
        RESPONSE_CANCELLED => Err(PastePortalError::Denied),
        other => Err(PastePortalError::failed(format!(
            "portal reported error response {other}"
        ))),
    }
}

/// 生成符合门户要求的句柄 token（对象路径元素只允许字母、数字和下划线）。
fn next_handle_token() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!("power_paste_paste_{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// 门户约定：请求句柄为 `<门户路径>/request/<发送方>/<token>`，
/// 发送方是总线唯一名去掉冒号并把点替换成下划线。
fn portal_handle_path(
    connection: &Connection,
    kind: &str,
    token: &str,
) -> Result<OwnedObjectPath> {
    let sender = connection
        .unique_name()
        .context("missing unique bus name")?
        .as_str()
        .trim_start_matches(':')
        .replace('.', "_");

    OwnedObjectPath::try_from(format!("{PORTAL_OBJECT_PATH}/{kind}/{sender}/{token}"))
        .context("invalid portal handle path")
}

fn portal_proxy<'a>(
    connection: &'a Connection,
    path: &'a str,
    interface: &'a str,
) -> Result<Proxy<'a>> {
    Proxy::new(connection, PORTAL_BUS_NAME, path, interface)
        .with_context(|| format!("failed to create {interface} proxy"))
}

/// 会话总线错误里命中这些关键字时，说明桌面没有可用的 RemoteDesktop 门户。
const PORTAL_UNAVAILABLE_HINTS: [&str; 6] = [
    "serviceunknown",
    "unknowninterface",
    "unknownmethod",
    "namehasnoowner",
    "notsupported",
    "no such interface",
];

fn classify_connection_error(error: zbus::Error) -> PastePortalError {
    PastePortalError::Unavailable(error.to_string())
}

/// 判断会话总线错误是否意味着门户不可用（服务 / 接口缺失）。
fn classify_bus_error(error: zbus::Error) -> PastePortalError {
    classify_bus_error_text(&error.to_string())
}

fn classify_bus_error_text(text: &str) -> PastePortalError {
    let lowercased = text.to_lowercase();
    if PORTAL_UNAVAILABLE_HINTS
        .iter()
        .any(|hint| lowercased.contains(hint))
    {
        PastePortalError::Unavailable(text.to_string())
    } else {
        PastePortalError::Failed(text.to_string())
    }
}

/// 门户返回的字符串可能是 `s`，也可能是被包了一层的变体。
fn value_to_string(value: &Value<'_>) -> Option<String> {
    match value {
        Value::Str(text) => Some(text.as_str().to_string()),
        Value::ObjectPath(path) => Some(path.as_str().to_string()),
        Value::Value(inner) => value_to_string(inner),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_tokens_are_valid_object_path_elements() {
        let first = next_handle_token();
        let second = next_handle_token();

        assert_ne!(first, second);
        assert!(first.starts_with("power_paste_paste_"));
        assert!(first
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || value == '_'));
    }

    #[test]
    fn injected_shortcut_is_control_v() {
        assert_eq!(KEYSYM_CONTROL_LEFT, 0xffe3);
        assert_eq!(KEYSYM_V, 0x76);
        assert_ne!(KEY_STATE_PRESSED, KEY_STATE_RELEASED);
        // 只申请键盘，避免多余的指针 / 触屏授权。
        assert_eq!(DEVICE_KEYBOARD, 1);
    }

    #[test]
    fn failures_map_to_frontend_message_keys() {
        assert_eq!(
            PastePortalError::Unavailable("no portal".into()).message_key(),
            "paste_portal_unavailable"
        );
        assert_eq!(PastePortalError::Denied.message_key(), "paste_portal_denied");
        assert_eq!(
            PastePortalError::Failed("boom".into()).message_key(),
            "paste_portal_failed"
        );
        assert_eq!(PastePortalError::Denied.detail(), "denied by the user");
    }

    #[test]
    fn missing_remote_desktop_interface_counts_as_unavailable() {
        assert!(matches!(
            classify_bus_error_text(
                "org.freedesktop.DBus.Error.UnknownMethod: Unknown method SelectDevices"
            ),
            PastePortalError::Unavailable(_)
        ));
        assert!(matches!(
            classify_bus_error_text("org.freedesktop.portal.Error.Failed: broken"),
            PastePortalError::Failed(_)
        ));
    }
}
