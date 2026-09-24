//! Wayland 会话下的全局快捷键后端。
//!
//! Wayland 合成器不会把按键交给 Xwayland 的全局抓键（XGrabKey），所以 Linux
//! Wayland 会话改用 XDG Desktop Portal 的 `org.freedesktop.portal.GlobalShortcuts`
//! 接口：快捷键由桌面环境托管，按键被触发（或松开）时桌面通过 `Activated` /
//! `Deactivated` 信号通知应用，GNOME / KDE / Hyprland 等合成器均已实现该接口。
//! 注意 `Deactivated` 只表示快捷键组合不再成立：GNOME 在松开主键（例如 `）时就会
//! 发出该信号，此时修饰键可能仍被按住，因此快速粘贴的结束时机由前端判断。
//!
//! 门户绑定需要用户在系统弹窗中确认，因此绑定结果是异步的：注册阶段先返回
//! “处理中”的状态，绑定结束后再通过快捷键状态事件把最终结果推给前端。

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, LazyLock, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
use zbus::{
    blocking::{Connection, MessageIterator, Proxy},
    message::Type as MessageType,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
    MatchRule,
};

use crate::models::{
    SharedState, ShortcutIssueDto, ShortcutStatusDto, QUICK_PASTE_RELEASED_EVENT,
};

use super::{
    app_id::{check_current_app_id, AppIdCheck},
    store_and_emit_shortcut_status, GLOBAL_SHORTCUT_KEY, LAN_TRANSFER_SHORTCUT_KEY,
    QUICK_PASTE_SHORTCUT_KEY,
};

/// 门户快捷键 ID：重启后复用同一 ID，桌面环境据此恢复用户改过的按键。
pub(crate) const TOGGLE_SHORTCUT_ID: &str = "toggle-panel";
pub(crate) const QUICK_PASTE_SHORTCUT_ID: &str = "quick-paste";
pub(crate) const LAN_TRANSFER_SHORTCUT_ID: &str = "open-lan-transfer";

/// 门户绑定失败时写入快捷键状态的错误码，前端据此显示可操作提示。
const ERROR_PORTAL_UNAVAILABLE: &str = "wayland_portal_unavailable";
const ERROR_PORTAL_DENIED: &str = "wayland_portal_denied";
const ERROR_PORTAL_FAILED: &str = "wayland_portal_failed";
const ERROR_PORTAL_CLOSED: &str = "wayland_portal_closed";
// 门户按调用进程的应用标识（app id）保存快捷键，拿不到应用标识时会直接拒绝。
const ERROR_PORTAL_NO_APP_ID: &str = "wayland_portal_no_app_id";
// 应用标识存在但不符合桌面要求（GNOME 只接受反向域名格式）时同样会被直接丢弃，
// 错误码后面会拼上实际标识，便于在设置页说明到底是哪个标识不合法。
const ERROR_PORTAL_INVALID_APP_ID: &str = "wayland_portal_invalid_app_id";

/// GNOME 的 GlobalShortcuts 门户在调用进程没有应用标识时返回的错误文本。
const PORTAL_NO_APP_ID_HINT: &str = "app id is required";

const PORTAL_BUS_NAME: &str = "org.freedesktop.portal.Desktop";
const PORTAL_OBJECT_PATH: &str = "/org/freedesktop/portal/desktop";
const GLOBAL_SHORTCUTS_INTERFACE: &str = "org.freedesktop.portal.GlobalShortcuts";
/// 会话专用接口名：门户规范把快捷键信号挂在 `GlobalShortcuts` 上，部分实现
/// （含 GNOME）实际用会话接口发出，两者都要接受。
const GLOBAL_SHORTCUTS_SESSION_INTERFACE: &str =
    "org.freedesktop.portal.GlobalShortcutsSession";
const REQUEST_INTERFACE: &str = "org.freedesktop.portal.Request";
const SESSION_INTERFACE: &str = "org.freedesktop.portal.Session";

// org.freedesktop.portal.Request::Response 的状态码。
const RESPONSE_SUCCESS: u32 = 0;
const RESPONSE_CANCELLED: u32 = 1;

/// 长按快捷键时桌面可能持续发送 Activated（按键重复），呼出面板需要防抖，
/// 否则面板会在按住期间反复开关。快速粘贴依赖连续激活切换候选项，不做防抖。
const TOGGLE_REPEAT_GUARD: Duration = Duration::from_millis(200);

/// 桌面报告快速粘贴快捷键失活后，延迟一小段时间再转发给前端：桌面可能在松开主键
/// （例如 `）时就发出 Deactivated，而用户往往还按着修饰键连续敲击切换候选项，
/// 这段窗口用于吸收连续敲击，是否最终提交由前端结合修饰键状态判断。
const QUICK_PASTE_RELEASE_DELAY: Duration = Duration::from_millis(200);

/// 会话总线报错信息中命中这些关键字时，说明桌面没有可用的 GlobalShortcuts 门户。
const PORTAL_UNAVAILABLE_HINTS: [&str; 6] = [
    "serviceunknown",
    "unknowninterface",
    "unknownmethod",
    "namehasnoowner",
    "notsupported",
    "no such interface",
];

/// 门户方法返回的结果字典（`a{sv}`）。
type PortalResults = HashMap<String, OwnedValue>;
/// 门户方法接收的选项字典（`a{sv}`），键固定为静态字符串。
type PortalOptions = HashMap<&'static str, Value<'static>>;

/// 需要由桌面门户托管的快捷键。
#[derive(Debug, Clone)]
pub(crate) struct PortalShortcut {
    pub(crate) id: &'static str,
    pub(crate) settings_key: &'static str,
    pub(crate) description: String,
    pub(crate) trigger: Option<String>,
    pub(crate) action: ShortcutAction,
}

/// 快捷键触发后应用要执行的动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShortcutAction {
    TogglePanel,
    QuickPaste,
    LanTransfer,
}

/// 当前生效的门户会话。
#[derive(Default)]
struct ActiveSession {
    generation: u64,
    path: Option<OwnedObjectPath>,
}

/// 门户绑定失败的原因。
enum PortalFailure {
    /// 桌面没有可用的 GlobalShortcuts 门户，可退回 X11 抓键。
    Unavailable(String),
    /// 用户在系统弹窗中取消了绑定。
    Cancelled,
    /// 门户返回了其它错误。
    Failed(String),
}

impl PortalFailure {
    fn failed(error: impl std::fmt::Display) -> Self {
        Self::Failed(error.to_string())
    }
}

#[derive(Clone)]
struct PortalRuntime {
    commands: mpsc::Sender<PortalCommand>,
}

enum PortalCommand {
    Bind {
        shortcuts: Vec<PortalShortcut>,
        issues: Vec<ShortcutIssueDto>,
    },
    Close,
}

/// 进程内单例：应用是单实例，门户线程随进程存活。
static RUNTIME: LazyLock<Mutex<Option<PortalRuntime>>> = LazyLock::new(|| Mutex::new(None));

/// 请求桌面门户绑定快捷键（非阻塞）。
///
/// 返回的状态只包含解析阶段的问题：门户绑定结果会在稍后通过快捷键状态事件推送。
pub(crate) fn bind_shortcuts(
    app: &AppHandle,
    shortcuts: Vec<PortalShortcut>,
    issues: Vec<ShortcutIssueDto>,
) -> ShortcutStatusDto {
    if let Some(status) = app_id_failure_status(&shortcuts, issues.clone(), check_current_app_id()) {
        return status;
    }

    let status = pending_status(issues.clone());

    match runtime(app) {
        Ok(runtime) => {
            let command = PortalCommand::Bind { shortcuts, issues };
            if let Err(error) = runtime.commands.send(command) {
                if let PortalCommand::Bind { shortcuts, issues } = error.0 {
                    handle_failure(
                        app,
                        &shortcuts,
                        issues,
                        PortalFailure::Unavailable("portal worker stopped".to_string()),
                    );
                }
            }
        }
        Err(error) => handle_failure(
            app,
            &shortcuts,
            issues,
            PortalFailure::Unavailable(error.to_string()),
        ),
    }

    status
}

/// 调用门户前自检进程应用标识：标识缺失或（GNOME 下）不是反向域名格式时，
/// 门户会直接丢弃绑定请求、连确认窗口都不弹，这里提前给出可操作提示，
/// 不再发起注定失败的绑定。
///
/// 返回 None 表示标识可用，或者在当前桌面上不构成阻断（例如 KDE / wlroots
/// 的实现并未校验格式，应交由门户自行判断）。
fn app_id_failure_status(
    shortcuts: &[PortalShortcut],
    issues: Vec<ShortcutIssueDto>,
    check: AppIdCheck,
) -> Option<ShortcutStatusDto> {
    // 清空快捷键时只是解绑，不需要应用标识。
    if shortcuts.is_empty() {
        return None;
    }

    let code = match check {
        AppIdCheck::Acceptable => return None,
        AppIdCheck::Missing => ERROR_PORTAL_NO_APP_ID.to_string(),
        AppIdCheck::Invalid(app_id) => format!("{ERROR_PORTAL_INVALID_APP_ID}:{app_id}"),
    };
    eprintln!("[shortcuts] 跳过门户绑定，进程应用标识不可用：{code}");

    let mut status = pending_status(issues);
    for shortcut in shortcuts {
        push_issue(&mut status, shortcut.settings_key, &code);
    }
    Some(status)
}

/// 关闭当前门户会话（更换或清空快捷键时先解绑）。
pub(crate) fn close_session(app: &AppHandle) {
    if let Ok(runtime) = runtime(app) {
        let _ = runtime.commands.send(PortalCommand::Close);
    }
}

/// 把应用内保存的快捷键转换成 XDG shortcuts 规范使用的触发器写法，
/// 例如 `Ctrl+Shift+V` → `<Control><Shift>v`。
pub(crate) fn trigger_from_shortcut(shortcut: &Shortcut) -> Option<String> {
    let mut trigger = String::new();
    if shortcut.mods.contains(Modifiers::CONTROL) {
        trigger.push_str("<Control>");
    }
    if shortcut.mods.contains(Modifiers::ALT) {
        trigger.push_str("<Alt>");
    }
    if shortcut.mods.contains(Modifiers::SHIFT) {
        trigger.push_str("<Shift>");
    }
    if shortcut.mods.contains(Modifiers::SUPER) {
        trigger.push_str("<Super>");
    }

    trigger.push_str(&keysym_name(shortcut.key)?);
    Some(trigger)
}

/// 取出门户运行时；首次调用会启动后台线程。
fn runtime(app: &AppHandle) -> Result<PortalRuntime> {
    let mut guard = RUNTIME.lock().unwrap();
    if let Some(runtime) = guard.as_ref() {
        return Ok(runtime.clone());
    }

    let (commands, receiver) = mpsc::channel();
    let worker_app = app.clone();
    thread::Builder::new()
        .name("wayland-portal-worker".into())
        .spawn(move || worker_loop(worker_app, receiver))
        .context("failed to start wayland portal worker")?;

    let runtime = PortalRuntime { commands };
    *guard = Some(runtime.clone());
    Ok(runtime)
}

/// 门户线程：串行处理绑定/解绑请求，避免两次绑定互相穿插。
fn worker_loop(app: AppHandle, receiver: mpsc::Receiver<PortalCommand>) {
    let connection = match Connection::session() {
        Ok(connection) => connection,
        Err(error) => {
            // 没有会话总线时门户不可用，后续请求统一回报失败。
            let detail = error.to_string();
            for command in receiver {
                if let PortalCommand::Bind { shortcuts, issues } = command {
                    handle_failure(
                        &app,
                        &shortcuts,
                        issues,
                        PortalFailure::Unavailable(detail.clone()),
                    );
                }
            }
            return;
        }
    };

    let session = Arc::new(Mutex::new(ActiveSession::default()));
    for command in receiver {
        match command {
            PortalCommand::Bind { shortcuts, issues } => {
                // 先写入“处理中”的状态：门户结果异步到达，这里先把上一次的失败提示清掉。
                publish_status(&app, pending_status(issues.clone()));

                if shortcuts.is_empty() {
                    close_active_session(&connection, &session);
                    continue;
                }

                match bind_session(&connection, &session, &shortcuts) {
                    Ok((generation, bound_ids)) => {
                        let mut status = status_from_shortcuts(&shortcuts, &bound_ids);
                        status.issues.splice(0..0, issues);
                        spawn_listener(
                            app.clone(),
                            connection.clone(),
                            session.clone(),
                            generation,
                            shortcuts,
                        );
                        eprintln!(
                            "[shortcuts] 桌面门户已托管快捷键：{}",
                            if bound_ids.is_empty() {
                                "无".to_string()
                            } else {
                                bound_ids.join("、")
                            }
                        );
                        publish_status(&app, status);
                    }
                    Err(failure) => handle_failure(&app, &shortcuts, issues, failure),
                }
            }
            PortalCommand::Close => {
                close_active_session(&connection, &session);
                publish_status(&app, ShortcutStatusDto::default());
            }
        }
    }
}

/// 新建门户会话并绑定快捷键，返回会话版本号与桌面实际绑定成功的快捷键 ID。
fn bind_session(
    connection: &Connection,
    session: &Arc<Mutex<ActiveSession>>,
    shortcuts: &[PortalShortcut],
) -> std::result::Result<(u64, Vec<String>), PortalFailure> {
    // 先关闭旧会话：同一个应用的两次绑定不能同时生效，否则两次信号会互相抢键。
    close_active_session(connection, session);

    let session_path = create_session(connection)?;
    let bound_ids = bind_shortcuts_in_session(connection, &session_path, shortcuts)?;

    let generation = {
        let mut guard = session.lock().unwrap();
        guard.generation += 1;
        guard.path = Some(session_path);
        guard.generation
    };

    Ok((generation, bound_ids))
}

/// 关闭当前门户会话并清空记录。
fn close_active_session(connection: &Connection, session: &Arc<Mutex<ActiveSession>>) {
    let path = {
        let mut guard = session.lock().unwrap();
        guard.generation += 1;
        guard.path.take()
    };

    let Some(path) = path else {
        return;
    };

    // 关闭失败说明会话已经失效，继续清理本地状态即可。
    let _ = close_portal_session(connection, &path);
}

/// 新建一个 GlobalShortcuts 会话。
fn create_session(connection: &Connection) -> std::result::Result<OwnedObjectPath, PortalFailure> {
    let handle_token = next_handle_token();
    let options = PortalOptions::from([
        ("handle_token", Value::from(handle_token.clone())),
        ("session_handle_token", Value::from(next_handle_token())),
    ]);

    let results = call_portal_request(connection, "CreateSession", &(options,), &handle_token)?;
    let handle = results
        .get("session_handle")
        .and_then(|value| value_to_string(value))
        .ok_or_else(|| PortalFailure::failed("portal response missing session handle"))?;
    OwnedObjectPath::try_from(handle)
        .map_err(|error| PortalFailure::failed(format!("invalid session handle: {error}")))
}

/// 在会话中绑定快捷键，返回桌面实际绑定成功的快捷键 ID。
fn bind_shortcuts_in_session(
    connection: &Connection,
    session_path: &OwnedObjectPath,
    shortcuts: &[PortalShortcut],
) -> std::result::Result<Vec<String>, PortalFailure> {
    let handle_token = next_handle_token();
    let body = bind_shortcuts_body(session_path, shortcuts, handle_token.clone());

    let results = call_portal_request(connection, "BindShortcuts", &body, &handle_token)?;

    // 结果里没有 shortcuts 字段时，按门户接受了全部请求处理。
    Ok(match results.get("shortcuts") {
        Some(value) => bound_shortcut_ids(value),
        None => shortcuts
            .iter()
            .map(|shortcut| shortcut.id.to_string())
            .collect(),
    })
}

/// 组装 BindShortcuts 的入参：门户约定为 `(o, a(sa{sv}), s, a{sv})`，
/// 即会话句柄、快捷键列表、父窗口标识与选项字典。
fn bind_shortcuts_body(
    session_path: &OwnedObjectPath,
    shortcuts: &[PortalShortcut],
    handle_token: String,
) -> (
    OwnedObjectPath,
    Vec<(String, PortalOptions)>,
    String,
    PortalOptions,
) {
    let options = PortalOptions::from([("handle_token", Value::from(handle_token))]);
    let entries = shortcuts
        .iter()
        .map(|shortcut| {
            let mut properties = PortalOptions::new();
            properties.insert("description", Value::from(shortcut.description.clone()));
            if let Some(trigger) = shortcut.trigger.clone() {
                properties.insert("preferred_trigger", Value::from(trigger));
            }
            (shortcut.id.to_string(), properties)
        })
        .collect::<Vec<_>>();

    (
        session_path.clone(),
        entries,
        // 没有父窗口：门户用系统级弹窗询问用户。
        String::new(),
        options,
    )
}

/// 调用门户方法并等待对应的 Request::Response 信号。
fn call_portal_request<B>(
    connection: &Connection,
    method: &str,
    body: &B,
    handle_token: &str,
) -> std::result::Result<PortalResults, PortalFailure>
where
    B: serde::Serialize + zbus::zvariant::DynamicType,
{
    let request_path = portal_handle_path(connection, "request", handle_token)
        .map_err(PortalFailure::failed)?;
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

    let proxy = portal_proxy(connection, PORTAL_OBJECT_PATH, GLOBAL_SHORTCUTS_INTERFACE)
        .map_err(PortalFailure::failed)?;
    proxy
        .call_method(method, body)
        .map_err(classify_bus_error)?;

    let message = responses
        .next()
        .ok_or_else(|| PortalFailure::failed("portal response stream closed"))?
        .map_err(classify_bus_error)?;
    let (code, results) = message
        .body()
        .deserialize::<(u32, PortalResults)>()
        .map_err(PortalFailure::failed)?;

    match code {
        RESPONSE_SUCCESS => Ok(results),
        RESPONSE_CANCELLED => Err(PortalFailure::Cancelled),
        other => Err(PortalFailure::failed(format!(
            "portal reported error response {other}"
        ))),
    }
}

/// 关闭会话对象，让桌面释放快捷键绑定。
fn close_portal_session(
    connection: &Connection,
    session_path: &OwnedObjectPath,
) -> std::result::Result<(), PortalFailure> {
    let proxy = portal_proxy(connection, session_path.as_str(), SESSION_INTERFACE)
        .map_err(PortalFailure::failed)?;
    proxy
        .call_method("Close", &())
        .map_err(classify_bus_error)?;
    Ok(())
}

/// 监听会话内的快捷键信号，实现面板呼出与快速粘贴。
fn spawn_listener(
    app: AppHandle,
    connection: Connection,
    session: Arc<Mutex<ActiveSession>>,
    generation: u64,
    shortcuts: Vec<PortalShortcut>,
) {
    let session_path = {
        let guard = session.lock().unwrap();
        guard.path.clone()
    };
    let Some(session_path) = session_path else {
        return;
    };

    let spawn_result = thread::Builder::new()
        .name("wayland-portal-listener".into())
        .spawn(move || {
            listen_session(
                app,
                connection,
                session,
                generation,
                session_path,
                shortcuts,
            )
        });

    if let Err(error) = spawn_result {
        eprintln!("[shortcuts] failed to start wayland portal listener: {error}");
    }
}

/// 门户信号循环：`Activated` 呼出面板，`Deactivated` 触发快速粘贴提交，
/// 会话被桌面关闭时把快捷键状态标记为未注册。
fn listen_session(
    app: AppHandle,
    connection: Connection,
    session: Arc<Mutex<ActiveSession>>,
    generation: u64,
    session_path: OwnedObjectPath,
    shortcuts: Vec<PortalShortcut>,
) {
    let rule = match MatchRule::builder()
        .msg_type(MessageType::Signal)
        .path_namespace(PORTAL_OBJECT_PATH)
    {
        Ok(builder) => builder.build(),
        Err(error) => {
            eprintln!("[shortcuts] failed to build portal match rule: {error}");
            return;
        }
    };
    let mut messages = match MessageIterator::for_match_rule(rule, &connection, None) {
        Ok(messages) => messages,
        Err(error) => {
            eprintln!("[shortcuts] failed to listen for portal signals: {error}");
            return;
        }
    };
    let mut last_toggle_at: Option<Instant> = None;
    let quick_paste_serial = Arc::new(AtomicU64::new(0));

    for message in messages.by_ref() {
        let Ok(message) = message else {
            break;
        };
        let interface = message
            .header()
            .interface()
            .map(|value| value.as_str().to_string());
        let member = message
            .header()
            .member()
            .map(|value| value.as_str().to_string());

        match (interface.as_deref(), member.as_deref()) {
            (Some(GLOBAL_SHORTCUTS_INTERFACE), Some("Activated"))
            | (Some(GLOBAL_SHORTCUTS_INTERFACE), Some("Deactivated"))
            | (Some(GLOBAL_SHORTCUTS_SESSION_INTERFACE), Some("Activated"))
            | (Some(GLOBAL_SHORTCUTS_SESSION_INTERFACE), Some("Deactivated")) => {
                let activated = member.as_deref() == Some("Activated");
                let Ok((signal_session, shortcut_id, _timestamp, _options)) = message
                    .body()
                    .deserialize::<(OwnedObjectPath, String, u64, PortalResults)>()
                else {
                    continue;
                };
                if signal_session.as_str() != session_path.as_str()
                    || !is_current_session(&session, generation)
                {
                    continue;
                }

                let Some(shortcut) = shortcuts.iter().find(|item| item.id == shortcut_id) else {
                    continue;
                };

                match (shortcut.action, activated) {
                    (ShortcutAction::TogglePanel, true) => {
                        let repeated = last_toggle_at
                            .map(|instant| instant.elapsed() < TOGGLE_REPEAT_GUARD)
                            .unwrap_or(false);
                        if repeated {
                            continue;
                        }
                        last_toggle_at = Some(Instant::now());
                        eprintln!("[shortcuts] 桌面门户触发：切换主面板");
                        let _ = crate::runtime::toggle_panel(&app);
                    }
                    (ShortcutAction::QuickPaste, true) => {
                        quick_paste_serial.fetch_add(1, Ordering::SeqCst);
                        let _ = crate::runtime::show_quick_paste_panel(&app);
                    }
                    (ShortcutAction::QuickPaste, false) => {
                        schedule_quick_paste_release(app.clone(), quick_paste_serial.clone());
                    }
                    (ShortcutAction::LanTransfer, true) => {
                        eprintln!("[shortcuts] 桌面门户触发：进入局域网互传");
                        let _ = crate::runtime::show_lan_transfer_panel(&app);
                    }
                    (ShortcutAction::LanTransfer, false) => {}
                    (ShortcutAction::TogglePanel, false) => {}
                }
            }
            (Some(SESSION_INTERFACE), Some("Closed")) => {
                let closed_path = message
                    .header()
                    .path()
                    .map(|value| value.as_str().to_string());
                if closed_path.as_deref() != Some(session_path.as_str()) {
                    continue;
                }

                handle_session_closed(&app, &session, generation, &session_path, &shortcuts);
                break;
            }
            _ => {}
        }
    }
}

/// 会话被桌面关闭后，把状态标记为未注册，让设置页提示用户重新绑定。
fn handle_session_closed(
    app: &AppHandle,
    session: &Arc<Mutex<ActiveSession>>,
    generation: u64,
    session_path: &OwnedObjectPath,
    shortcuts: &[PortalShortcut],
) {
    let is_current = {
        let mut guard = session.lock().unwrap();
        if guard.generation == generation && guard.path.as_ref() == Some(session_path) {
            guard.path = None;
            true
        } else {
            false
        }
    };
    if !is_current {
        return;
    }

    let mut status = ShortcutStatusDto::default();
    for shortcut in shortcuts {
        push_issue(&mut status, shortcut.settings_key, ERROR_PORTAL_CLOSED);
    }
    publish_status(app, status);
}

/// 会话是否仍是最新一次绑定的结果（旧监听线程收到信号时直接忽略）。
fn is_current_session(session: &Arc<Mutex<ActiveSession>>, generation: u64) -> bool {
    session.lock().unwrap().generation == generation
}

/// 门户托管时按键不一定进入面板，桌面只能通过 Deactivated 报告快捷键已失活，
/// 且该信号可能在主键（例如 `）抬起、修饰键仍按住时到达。这里延迟一小段时间
/// 再把“失活”转发给前端：期间用户再次按下快捷键（连续敲击切换候选项）会取消
/// 本次转发，最终是否提交由前端结合修饰键状态判断。
fn schedule_quick_paste_release(app: AppHandle, serial: Arc<AtomicU64>) {
    let pending = serial.load(Ordering::SeqCst);
    let spawn_result = thread::Builder::new()
        .name("wayland-portal-quick-paste-release".into())
        .spawn(move || {
            thread::sleep(QUICK_PASTE_RELEASE_DELAY);
            if serial.load(Ordering::SeqCst) != pending {
                return;
            }
            eprintln!("[shortcuts] 桌面门户触发：快速粘贴快捷键失活");
            let _ = app.emit(QUICK_PASTE_RELEASED_EVENT, ());
        });

    if let Err(error) = spawn_result {
        eprintln!("[shortcuts] failed to schedule quick paste release: {error}");
    }
}

/// 绑定失败：记录原因，门户不可用时退回 X11 抓键。
fn handle_failure(
    app: &AppHandle,
    shortcuts: &[PortalShortcut],
    issues: Vec<ShortcutIssueDto>,
    failure: PortalFailure,
) {
    let (code, detail) = match &failure {
        PortalFailure::Unavailable(error) => (ERROR_PORTAL_UNAVAILABLE, error.to_string()),
        PortalFailure::Cancelled => (ERROR_PORTAL_DENIED, "cancelled by user".to_string()),
        PortalFailure::Failed(error) => (failed_error_code(error), error.to_string()),
    };
    eprintln!("[shortcuts] wayland portal binding failed: {code}: {detail}");

    if let PortalFailure::Unavailable(_) = failure {
        // 桌面没有全局快捷键门户：退回 X11 抓键，保证行为不比旧版本差。
        register_with_plugin_backend(app, shortcuts, issues, code);
        return;
    }

    let mut status = pending_status(issues);
    for shortcut in shortcuts {
        push_issue(&mut status, shortcut.settings_key, code);
    }
    publish_status(app, status);
}

/// 缺少应用标识时前端要提示“如何让桌面识别应用”，而不是笼统的绑定失败。
fn failed_error_code(error: &str) -> &'static str {
    if error.to_lowercase().contains(PORTAL_NO_APP_ID_HINT) {
        ERROR_PORTAL_NO_APP_ID
    } else {
        ERROR_PORTAL_FAILED
    }
}

/// 门户不可用时退回 tauri-plugin-global-shortcut。
fn register_with_plugin_backend(
    app: &AppHandle,
    shortcuts: &[PortalShortcut],
    issues: Vec<ShortcutIssueDto>,
    code: &str,
) {
    let Some(state) = app.try_state::<Arc<SharedState>>() else {
        return;
    };
    let settings = state.settings.lock().unwrap().clone();
    let mut status = super::register_shortcuts_with_plugin(app, &settings);
    status.issues.splice(0..0, issues);
    for shortcut in shortcuts {
        push_issue(&mut status, shortcut.settings_key, code);
    }
    store_and_emit_shortcut_status(app, state.inner(), status);
}

/// 绑定进行中的状态：只保留解析阶段的问题，注册结果稍后异步回报。
fn pending_status(issues: Vec<ShortcutIssueDto>) -> ShortcutStatusDto {
    ShortcutStatusDto {
        global_shortcut_registered: false,
        quick_paste_shortcut_registered: false,
        issues,
    }
}

fn push_issue(status: &mut ShortcutStatusDto, settings_key: &str, code: &str) {
    if status.issues.iter().any(|issue| issue.key == settings_key) {
        return;
    }

    status.issues.push(ShortcutIssueDto {
        key: settings_key.to_string(),
        shortcut: String::new(),
        error: code.to_string(),
    });
}

fn publish_status(app: &AppHandle, status: ShortcutStatusDto) {
    let Some(state) = app.try_state::<Arc<SharedState>>() else {
        return;
    };
    store_and_emit_shortcut_status(app, state.inner(), status);
}

/// 依据实际绑定成功的快捷键 ID 生成状态，未绑定成功的快捷键记录问题。
fn status_from_shortcuts(shortcuts: &[PortalShortcut], bound_ids: &[String]) -> ShortcutStatusDto {
    let mut status = ShortcutStatusDto::default();
    for shortcut in shortcuts {
        if bound_ids.iter().any(|bound_id| bound_id == shortcut.id) {
            match shortcut.settings_key {
                GLOBAL_SHORTCUT_KEY => status.global_shortcut_registered = true,
                QUICK_PASTE_SHORTCUT_KEY => status.quick_paste_shortcut_registered = true,
                LAN_TRANSFER_SHORTCUT_KEY => status.lan_transfer_shortcut_registered = true,
                _ => {}
            }
            continue;
        }

        push_issue(&mut status, shortcut.settings_key, ERROR_PORTAL_DENIED);
    }
    status
}

/// 从 BindShortcuts 的结果里取出已绑定的快捷键 ID。
fn bound_shortcut_ids(value: &Value<'_>) -> Vec<String> {
    let Value::Array(entries) = value else {
        return Vec::new();
    };

    entries
        .iter()
        .filter_map(|entry| match entry {
            Value::Structure(structure) => structure.fields().first().and_then(value_to_string),
            _ => None,
        })
        .collect()
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

/// 生成符合门户要求的句柄 token（对象路径元素只允许字母、数字和下划线）。
fn next_handle_token() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!("power_paste_{}", COUNTER.fetch_add(1, Ordering::Relaxed))
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

/// 判断会话总线错误是否意味着门户不可用（服务/接口缺失）。
fn classify_bus_error(error: zbus::Error) -> PortalFailure {
    let text = error.to_string().to_lowercase();
    if PORTAL_UNAVAILABLE_HINTS
        .iter()
        .any(|hint| text.contains(hint))
    {
        PortalFailure::Unavailable(error.to_string())
    } else {
        PortalFailure::Failed(error.to_string())
    }
}

/// 把按键转换成 X11 keysym 名称：XDG shortcuts 规范用 keysym 描述按键。
fn keysym_name(key: Code) -> Option<String> {
    let name = key.to_string();

    if let Some(letter) = name.strip_prefix("Key") {
        return (letter.len() == 1 && letter.chars().all(|value| value.is_ascii_alphabetic()))
            .then(|| letter.to_ascii_lowercase());
    }
    if let Some(digit) = name.strip_prefix("Digit") {
        return (digit.len() == 1 && digit.chars().all(|value| value.is_ascii_digit()))
            .then(|| digit.to_string());
    }
    if let Some(number) = name.strip_prefix('F') {
        if !number.is_empty() && number.chars().all(|value| value.is_ascii_digit()) {
            // F1..F24 的 keysym 名称与 Code 名称一致。
            return Some(name);
        }
    }

    let keysym = match key {
        Code::Backquote => "grave",
        Code::Backslash => "backslash",
        Code::BracketLeft => "bracketleft",
        Code::BracketRight => "bracketright",
        Code::Comma => "comma",
        Code::Equal => "equal",
        Code::Minus => "minus",
        Code::Period => "period",
        Code::Quote => "apostrophe",
        Code::Semicolon => "semicolon",
        Code::Slash => "slash",
        Code::Backspace => "BackSpace",
        Code::Delete => "Delete",
        Code::End => "End",
        Code::Enter => "Return",
        Code::Escape => "Escape",
        Code::Home => "Home",
        Code::Insert => "Insert",
        Code::PageDown => "Next",
        Code::PageUp => "Prior",
        Code::Space => "space",
        Code::Tab => "Tab",
        Code::ArrowDown => "Down",
        Code::ArrowLeft => "Left",
        Code::ArrowRight => "Right",
        Code::ArrowUp => "Up",
        // 其它按键无法可靠映射到 keysym，交由桌面让用户自行指定。
        _ => return None,
    };

    Some(keysym.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::ObjectPath;

    fn shortcut(value: &str) -> Shortcut {
        value.parse::<Shortcut>().expect("valid shortcut")
    }

    fn portal_shortcut(id: &'static str, settings_key: &'static str) -> PortalShortcut {
        PortalShortcut {
            id,
            settings_key,
            description: String::new(),
            trigger: None,
            action: ShortcutAction::TogglePanel,
        }
    }

    #[test]
    fn trigger_uses_xdg_shortcut_syntax() {
        assert_eq!(
            trigger_from_shortcut(&shortcut("Ctrl+Shift+V")).as_deref(),
            Some("<Control><Shift>v")
        );
        assert_eq!(
            trigger_from_shortcut(&shortcut("Ctrl+Backquote")).as_deref(),
            Some("<Control>grave")
        );
        assert_eq!(
            trigger_from_shortcut(&shortcut("Super+Space")).as_deref(),
            Some("<Super>space")
        );
        assert_eq!(
            trigger_from_shortcut(&shortcut("Ctrl+Alt+F9")).as_deref(),
            Some("<Control><Alt>F9")
        );
        assert_eq!(
            trigger_from_shortcut(&shortcut("Ctrl+1")).as_deref(),
            Some("<Control>1")
        );
        assert_eq!(
            trigger_from_shortcut(&shortcut("Ctrl+Escape")).as_deref(),
            Some("<Control>Escape")
        );
    }

    #[test]
    fn trigger_skips_keys_without_keysym_mapping() {
        assert_eq!(trigger_from_shortcut(&shortcut("Ctrl+MediaPlay")), None);
    }

    #[test]
    fn status_marks_only_bound_shortcuts() {
        let shortcuts = [
            portal_shortcut(TOGGLE_SHORTCUT_ID, GLOBAL_SHORTCUT_KEY),
            portal_shortcut(QUICK_PASTE_SHORTCUT_ID, QUICK_PASTE_SHORTCUT_KEY),
        ];

        let both = status_from_shortcuts(
            &shortcuts,
            &[
                TOGGLE_SHORTCUT_ID.to_string(),
                QUICK_PASTE_SHORTCUT_ID.to_string(),
            ],
        );
        assert!(both.global_shortcut_registered);
        assert!(both.quick_paste_shortcut_registered);
        assert!(both.issues.is_empty());

        let toggle_only = status_from_shortcuts(&shortcuts, &[TOGGLE_SHORTCUT_ID.to_string()]);
        assert!(toggle_only.global_shortcut_registered);
        assert!(!toggle_only.quick_paste_shortcut_registered);
        assert_eq!(toggle_only.issues.len(), 1);
        assert_eq!(toggle_only.issues[0].key, QUICK_PASTE_SHORTCUT_KEY);
        assert_eq!(toggle_only.issues[0].error, ERROR_PORTAL_DENIED);

        let none = status_from_shortcuts(&shortcuts, &[]);
        assert!(!none.global_shortcut_registered);
        assert!(!none.quick_paste_shortcut_registered);
        assert_eq!(none.issues.len(), 2);
    }

    #[test]
    fn missing_app_id_failure_uses_dedicated_code() {
        assert_eq!(
            failed_error_code("org.freedesktop.portal.Error.NotAllowed: An app id is required"),
            ERROR_PORTAL_NO_APP_ID
        );
        assert_eq!(
            failed_error_code("org.freedesktop.portal.Error.Failed: unknown error"),
            ERROR_PORTAL_FAILED
        );
    }

    #[test]
    fn missing_app_id_skips_the_portal_and_explains_why() {
        let shortcuts = [
            portal_shortcut(TOGGLE_SHORTCUT_ID, GLOBAL_SHORTCUT_KEY),
            portal_shortcut(QUICK_PASTE_SHORTCUT_ID, QUICK_PASTE_SHORTCUT_KEY),
        ];

        let status = app_id_failure_status(&shortcuts, Vec::new(), AppIdCheck::Missing)
            .expect("missing app id must not reach the portal");

        assert!(!status.global_shortcut_registered);
        assert!(!status.quick_paste_shortcut_registered);
        assert_eq!(status.issues.len(), 2);
        assert!(status
            .issues
            .iter()
            .all(|issue| issue.error == ERROR_PORTAL_NO_APP_ID));
    }

    #[test]
    fn invalid_app_id_is_reported_with_the_identifier() {
        let shortcuts = [portal_shortcut(TOGGLE_SHORTCUT_ID, GLOBAL_SHORTCUT_KEY)];

        let status = app_id_failure_status(
            &shortcuts,
            Vec::new(),
            AppIdCheck::Invalid("power-paste".to_string()),
        )
        .expect("invalid app id must not reach the portal");

        assert_eq!(status.issues.len(), 1);
        assert_eq!(
            status.issues[0].error,
            "wayland_portal_invalid_app_id:power-paste"
        );
    }

    #[test]
    fn acceptable_app_id_and_empty_shortcuts_keep_using_the_portal() {
        let shortcuts = [portal_shortcut(TOGGLE_SHORTCUT_ID, GLOBAL_SHORTCUT_KEY)];

        assert!(app_id_failure_status(&shortcuts, Vec::new(), AppIdCheck::Acceptable).is_none());
        // 清空快捷键只是解绑，即使没有应用标识也要让门户关闭旧会话。
        assert!(app_id_failure_status(&[], Vec::new(), AppIdCheck::Missing).is_none());
    }

    #[test]
    fn response_values_are_read_as_strings() {
        assert_eq!(
            value_to_string(&Value::from("session")),
            Some("session".to_string())
        );
        let path = ObjectPath::try_from("/org/freedesktop/portal/desktop/session/1/token")
            .expect("valid object path");
        assert_eq!(
            value_to_string(&Value::ObjectPath(path)),
            Some("/org/freedesktop/portal/desktop/session/1/token".to_string())
        );
        assert_eq!(value_to_string(&Value::from(1u32)), None);
    }

    #[test]
    fn handle_tokens_are_valid_object_path_elements() {
        let first = next_handle_token();
        let second = next_handle_token();

        assert_ne!(first, second);
        assert!(first
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || value == '_'));
    }

    #[test]
    fn bind_shortcuts_arguments_match_portal_signature() {
        use zbus::zvariant::{DynamicType, Signature};

        let session = OwnedObjectPath::try_from("/org/freedesktop/portal/desktop/session/1/p")
            .expect("valid session path");
        let shortcuts = [PortalShortcut {
            id: TOGGLE_SHORTCUT_ID,
            settings_key: GLOBAL_SHORTCUT_KEY,
            description: "Show main panel".to_string(),
            trigger: Some("<Control><Shift>v".to_string()),
            action: ShortcutAction::TogglePanel,
        }];

        let body = bind_shortcuts_body(&session, &shortcuts, "power_paste_1".to_string());

        assert_eq!(
            body.signature(),
            Signature::try_from("(oa(sa{sv})sa{sv})").expect("portal signature")
        );
        assert_eq!(body.0.as_str(), session.as_str());
        assert_eq!(body.2, "");

        let (shortcut_id, properties) = &body.1[0];
        assert_eq!(shortcut_id, TOGGLE_SHORTCUT_ID);
        assert_eq!(
            properties.get("description"),
            Some(&Value::from("Show main panel"))
        );
        assert_eq!(
            properties.get("preferred_trigger"),
            Some(&Value::from("<Control><Shift>v"))
        );
        assert_eq!(
            body.3.get("handle_token"),
            Some(&Value::from("power_paste_1"))
        );
    }
}
