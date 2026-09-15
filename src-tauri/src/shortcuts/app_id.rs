//! Linux 下进程应用标识（app id）的自检。
//!
//! Wayland 会话的全局快捷键由桌面门户托管，门户按调用进程 cgroup 里
//! `app-*.scope` 的单元名推导应用标识；GNOME 侧还会用 GLib 的
//! `g_application_id_is_valid()` 再校验一次，要求反向域名格式（至少包含一个点）。
//! 标识不合法时 GNOME 会直接丢弃绑定请求、连确认窗口都不弹，用户只看到
//! “绑定失败”，因此这里在调用门户之前先把标识算出来，把这类失败提前变成
//! 可操作的提示。

use std::path::Path;

/// `/proc/self/cgroup` 里取出的应用标识，以及它是否还能被门户接受。
pub(crate) enum AppIdCheck {
    /// 标识合法，或者当前桌面不额外校验（例如 KDE / wlroots），照常交给门户。
    Acceptable,
    /// 进程没有应用标识（不在 app-*.scope 里），门户必然拒绝。
    Missing,
    /// 桌面（GNOME）只接受反向域名标识，当前标识会被直接丢弃。
    Invalid(String),
}

/// 读取当前进程的 cgroup 并推导应用标识。
pub(crate) fn current_app_id() -> Option<String> {
    let content = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    app_id_from_cgroup(&content)
}

/// 判断当前进程的应用标识能否交给门户绑定快捷键。
pub(crate) fn check_current_app_id() -> AppIdCheck {
    match current_app_id() {
        None => AppIdCheck::Missing,
        Some(app_id) => {
            if is_valid_app_id(&app_id) || !is_gnome_desktop() {
                AppIdCheck::Acceptable
            } else {
                AppIdCheck::Invalid(app_id)
            }
        }
    }
}

/// 从 `/proc/self/cgroup` 内容里取应用标识：cgroup 路径的最后一层就是进程
/// 所在的 systemd 单元名（cgroup v1 / v2 的行格式都是 `<层级>:<控制器>:<路径>`）。
fn app_id_from_cgroup(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let path = line.splitn(3, ':').nth(2)?;
        let unit = Path::new(path).file_name()?.to_str()?;
        app_id_from_unit_name(unit)
    })
}

/// 按 XDG Desktop Portal 的规则从 systemd 单元名取应用标识：
/// `^app-(?:[[:alnum:]]+\-)?(.+?)(?:\-[[:alnum:]]*)(?:\.scope|\.slice)$`
///
/// 可选的 `[[:alnum:]]+-` 前缀是启动器名（GNOME 启动应用时写 `gnome-`，
/// `pnpm tauri dev` 写 `dev-`），末尾 `-<随机后缀>` 是 pid 或唯一后缀，
/// 两者都不属于应用标识。
fn app_id_from_unit_name(unit: &str) -> Option<String> {
    let body = unit.strip_prefix("app-")?;
    let body = body
        .strip_suffix(".scope")
        .or_else(|| body.strip_suffix(".slice"))?;

    let (body, unique_suffix) = body.rsplit_once('-')?;
    if unique_suffix.is_empty() || !unique_suffix.chars().all(|value| value.is_ascii_alphanumeric()) {
        return None;
    }

    Some(match body.split_once('-') {
        // 启动器前缀只在“全字母数字 + 连字符”时剥离，避免吃掉标识本身的一段。
        Some((prefix, rest))
            if !rest.is_empty()
                && prefix
                    .chars()
                    .all(|value| value.is_ascii_alphanumeric()) =>
        {
            rest.to_string()
        }
        _ => body.to_string(),
    })
}

/// 复刻 GLib `g_application_id_is_valid()`：GNOME 的 GlobalShortcuts 提供者
/// 用它校验门户传入的标识，不合法就直接丢弃绑定请求。
///
/// 规则（已在本机用 PyGObject 逐条核对）：非空、不超过 255 字节、首字符是字母或
/// `_`/`-`、字符集限 `[A-Za-z0-9._-]`、至少包含一个点、点不能出现在首尾、
/// 不能出现连续点。
pub(crate) fn is_valid_app_id(app_id: &str) -> bool {
    let bytes = app_id.as_bytes();
    if bytes.is_empty() || bytes.len() > 255 {
        return false;
    }

    let first = bytes[0];
    if !(first.is_ascii_alphabetic() || first == b'_' || first == b'-') {
        return false;
    }

    if !bytes.contains(&b'.') || app_id.ends_with('.') || app_id.contains("..") {
        return false;
    }

    bytes
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

/// 当前桌面是否为 GNOME：只有 GNOME 的 GlobalShortcuts 提供者会按 GLib 规则
/// 拒绝非反向域名标识，其它实现可能照常接受，因此不要替它们下结论。
fn is_gnome_desktop() -> bool {
    ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP"]
        .iter()
        .filter_map(|key| std::env::var(key).ok())
        .any(|value| value.to_ascii_lowercase().contains("gnome"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_id_comes_from_the_last_unit_in_the_cgroup() {
        let cgroup = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-gnome-org.mozilla.firefox-1234.scope\n";
        assert_eq!(app_id_from_cgroup(cgroup).as_deref(), Some("org.mozilla.firefox"));

        let nested = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-dev-com.yulei.powerpaste-4019648.scope\n";
        assert_eq!(
            app_id_from_cgroup(nested).as_deref(),
            Some("com.yulei.powerpaste")
        );

        let v1 = "1:name=systemd:/user.slice/user-1000.slice/user@1000.service/app.slice/app-foo-power-paste-13765.scope\n";
        assert_eq!(app_id_from_cgroup(v1).as_deref(), Some("power-paste"));
    }

    #[test]
    fn plain_session_scopes_have_no_app_id() {
        let cgroup = "0::/user.slice/user-1000.slice/user@1000.service/session.slice/session-2.scope\n";
        assert_eq!(app_id_from_cgroup(cgroup), None);

        let service = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.gnome.SettingsDaemon.MediaKeys@autostart.service\n";
        assert_eq!(app_id_from_cgroup(service), None);
    }

    #[test]
    fn desktop_file_ids_keep_their_launcher_prefix_out() {
        assert_eq!(
            app_id_from_unit_name("app-gnome-com.yulei.powerpaste-02443.scope").as_deref(),
            Some("com.yulei.powerpaste")
        );
        assert_eq!(
            app_id_from_unit_name("app-gnome-power-paste-22928.scope").as_deref(),
            Some("power-paste")
        );
        // 桌面项名带空格时 systemd 会把空格转义，标识因此非法。
        assert_eq!(
            app_id_from_unit_name("app-gnome-Power\\x20Paste-14966.scope").as_deref(),
            Some("Power\\x20Paste")
        );
    }

    #[test]
    fn glib_app_id_rules_are_mirrored() {
        assert!(is_valid_app_id("com.yulei.powerpaste"));
        assert!(is_valid_app_id("org.gnome.Nautilus"));
        assert!(is_valid_app_id("com.yulei_power"));
        assert!(is_valid_app_id("com.yulei-power-paste"));
        assert!(is_valid_app_id("_com.example"));
        assert!(is_valid_app_id("a.b"));

        assert!(!is_valid_app_id("power-paste"));
        assert!(!is_valid_app_id("1com.example"));
        assert!(!is_valid_app_id("com..example"));
        assert!(!is_valid_app_id(".com.example"));
        assert!(!is_valid_app_id("com.example."));
        assert!(!is_valid_app_id("Power Paste"));
        assert!(!is_valid_app_id("Power\\x20Paste"));
        assert!(!is_valid_app_id("com.示例"));
        assert!(!is_valid_app_id(""));
        assert!(!is_valid_app_id(&format!("com.{}", "x".repeat(260))));
    }
}
