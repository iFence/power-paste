use anyhow::Result;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

#[cfg(target_os = "linux")]
mod portal;

use crate::models::{
    AppSettings, ShortcutIssueDto, ShortcutStatusDto, SHORTCUT_STATUS_UPDATED_EVENT,
};

const GLOBAL_SHORTCUT_KEY: &str = "globalShortcut";
const QUICK_PASTE_SHORTCUT_KEY: &str = "quickPasteShortcut";

#[derive(Debug, Clone)]
struct ParsedShortcut {
    key: &'static str,
    value: String,
    shortcut: Shortcut,
}

fn parse_optional_shortcut(
    key: &'static str,
    value: &str,
    label: &str,
) -> std::result::Result<Option<ParsedShortcut>, ShortcutIssueDto> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }

    value
        .parse::<Shortcut>()
        .map(|shortcut| {
            Some(ParsedShortcut {
                key,
                value: value.to_string(),
                shortcut,
            })
        })
        .map_err(|error| ShortcutIssueDto {
            key: key.into(),
            shortcut: value.into(),
            error: format!("invalid_{label}: {error}"),
        })
}

fn parse_configured_shortcuts(
    settings: &AppSettings,
) -> (Vec<ParsedShortcut>, Vec<ShortcutIssueDto>) {
    let mut shortcuts = Vec::new();
    let mut issues = Vec::new();

    match parse_optional_shortcut(
        GLOBAL_SHORTCUT_KEY,
        &settings.global_shortcut,
        "global_shortcut",
    ) {
        Ok(Some(shortcut)) => shortcuts.push(shortcut),
        Ok(None) => {}
        Err(issue) => issues.push(issue),
    }

    match parse_optional_shortcut(
        QUICK_PASTE_SHORTCUT_KEY,
        &settings.quick_paste_shortcut,
        "quick_paste_shortcut",
    ) {
        Ok(Some(shortcut)) => shortcuts.push(shortcut),
        Ok(None) => {}
        Err(issue) => issues.push(issue),
    }

    if shortcuts.len() == 2 && shortcuts[0].value == shortcuts[1].value {
        issues.push(ShortcutIssueDto {
            key: GLOBAL_SHORTCUT_KEY.into(),
            shortcut: shortcuts[0].value.clone(),
            error: "duplicate_shortcut".into(),
        });
        issues.push(ShortcutIssueDto {
            key: QUICK_PASTE_SHORTCUT_KEY.into(),
            shortcut: shortcuts[1].value.clone(),
            error: "duplicate_shortcut".into(),
        });
        shortcuts.clear();
    }

    (shortcuts, issues)
}

/// Wayland 合成器不会把按键交给 Xwayland 的全局抓键，只能交给桌面门户托管。
///
/// 门户绑定是异步的，状态由门户线程写入，同步调用方不应再覆盖它。
pub(crate) fn uses_wayland_portal() -> bool {
    #[cfg(target_os = "linux")]
    let wayland = crate::clipboard::linux_session_backend() == "wayland";
    #[cfg(not(target_os = "linux"))]
    let wayland = false;

    wayland
}

/// 桌面弹窗中展示的快捷键说明文案。
#[cfg(target_os = "linux")]
fn portal_descriptions(locale: &str) -> (&'static str, &'static str) {
    if locale == "zh-CN" {
        ("呼出主面板", "快速粘贴（按住呼出候选，松开立即粘贴）")
    } else {
        ("Show main panel", "Quick paste (hold to open, release to paste)")
    }
}

/// 把设置里的快捷键转换成门户需要的描述与触发器。
#[cfg(target_os = "linux")]
fn portal_shortcuts(shortcuts: &[ParsedShortcut], locale: &str) -> Vec<portal::PortalShortcut> {
    let (toggle_description, quick_paste_description) = portal_descriptions(locale);

    shortcuts
        .iter()
        .filter_map(|shortcut| {
            let (id, action, description) = match shortcut.key {
                GLOBAL_SHORTCUT_KEY => (
                    portal::TOGGLE_SHORTCUT_ID,
                    portal::ShortcutAction::TogglePanel,
                    toggle_description,
                ),
                QUICK_PASTE_SHORTCUT_KEY => (
                    portal::QUICK_PASTE_SHORTCUT_ID,
                    portal::ShortcutAction::QuickPaste,
                    quick_paste_description,
                ),
                _ => return None,
            };

            Some(portal::PortalShortcut {
                id,
                settings_key: shortcut.key,
                description: description.to_string(),
                // 无法映射到 keysym 的按键不传 preferred_trigger，
                // 由用户在桌面弹窗里自行指定。
                trigger: portal::trigger_from_shortcut(&shortcut.shortcut),
                action,
            })
        })
        .collect()
}

pub(crate) fn unregister_configured_shortcuts(app: &AppHandle, settings: &AppSettings) {
    let (shortcuts, _) = parse_configured_shortcuts(settings);

    #[cfg(target_os = "linux")]
    if uses_wayland_portal() {
        portal::close_session(app);
        return;
    }

    unregister_plugin_shortcuts(app, &shortcuts);
}

fn unregister_plugin_shortcuts(app: &AppHandle, shortcuts: &[ParsedShortcut]) {
    for shortcut in shortcuts {
        let _ = app.global_shortcut().unregister(shortcut.shortcut);
    }
}

/// 注册全局快捷键：Wayland 会话交给桌面门户，其它平台沿用抓键。
pub(crate) fn register_shortcuts_nonfatal(
    app: &AppHandle,
    settings: &AppSettings,
) -> ShortcutStatusDto {
    let (shortcuts, issues) = parse_configured_shortcuts(settings);

    #[cfg(target_os = "linux")]
    if uses_wayland_portal() {
        // 门户绑定需要用户在系统弹窗中确认，结果异步回报。
        return portal::bind_shortcuts(
            app,
            portal_shortcuts(&shortcuts, &settings.locale),
            issues,
        );
    }

    register_plugin_shortcuts(app, &shortcuts, issues)
}

fn register_plugin_shortcuts(
    app: &AppHandle,
    shortcuts: &[ParsedShortcut],
    mut issues: Vec<ShortcutIssueDto>,
) -> ShortcutStatusDto {
    let mut status = ShortcutStatusDto {
        issues: Vec::new(),
        ..ShortcutStatusDto::default()
    };

    for shortcut in shortcuts {
        match app.global_shortcut().register(shortcut.shortcut) {
            Ok(()) => mark_shortcut_registered(&mut status, shortcut.key),
            Err(error) => issues.push(ShortcutIssueDto {
                key: shortcut.key.into(),
                shortcut: shortcut.value.clone(),
                error: error.to_string(),
            }),
        }
    }

    status.issues = issues;
    status
}

/// 直接使用 tauri-plugin-global-shortcut 注册（门户不可用时回退）。
#[cfg(target_os = "linux")]
pub(crate) fn register_shortcuts_with_plugin(
    app: &AppHandle,
    settings: &AppSettings,
) -> ShortcutStatusDto {
    let (shortcuts, issues) = parse_configured_shortcuts(settings);
    register_plugin_shortcuts(app, &shortcuts, issues)
}

/// 严格注册：任一快捷键注册失败就回滚并返回错误。
pub(crate) fn register_shortcuts_strict(
    app: &AppHandle,
    settings: &AppSettings,
) -> Result<ShortcutStatusDto> {
    let (shortcuts, issues) = parse_configured_shortcuts(settings);
    if let Some(issue) = issues.first() {
        anyhow::bail!("{}", issue.error);
    }

    #[cfg(target_os = "linux")]
    if uses_wayland_portal() {
        // 门户绑定失败只会在异步结果里出现，这里不做同步回滚。
        return Ok(portal::bind_shortcuts(
            app,
            portal_shortcuts(&shortcuts, &settings.locale),
            Vec::new(),
        ));
    }

    register_plugin_shortcuts_strict(app, &shortcuts)
}

fn register_plugin_shortcuts_strict(
    app: &AppHandle,
    shortcuts: &[ParsedShortcut],
) -> Result<ShortcutStatusDto> {
    let mut registered = Vec::new();
    let mut status = ShortcutStatusDto::default();
    for shortcut in shortcuts {
        if let Err(error) = app.global_shortcut().register(shortcut.shortcut) {
            for registered_shortcut in registered {
                let _ = app.global_shortcut().unregister(registered_shortcut);
            }
            anyhow::bail!("shortcut_registration_failed:{}:{}", shortcut.key, error);
        }

        registered.push(shortcut.shortcut);
        mark_shortcut_registered(&mut status, shortcut.key);
    }

    Ok(status)
}

fn mark_shortcut_registered(status: &mut ShortcutStatusDto, key: &str) {
    if key == GLOBAL_SHORTCUT_KEY {
        status.global_shortcut_registered = true;
    } else if key == QUICK_PASTE_SHORTCUT_KEY {
        status.quick_paste_shortcut_registered = true;
    }
}

pub(crate) fn store_and_emit_shortcut_status(
    app: &AppHandle,
    state: &std::sync::Arc<crate::models::SharedState>,
    status: ShortcutStatusDto,
) {
    *state.shortcut_status.lock().unwrap() = status.clone();
    let _ = app.emit(SHORTCUT_STATUS_UPDATED_EVENT, status);
}

#[cfg(test)]
mod tests {
    use super::parse_configured_shortcuts;
    use crate::models::AppSettings;

    #[test]
    fn duplicate_shortcuts_become_status_issues() {
        let mut settings = AppSettings::default().normalized();
        settings.quick_paste_shortcut = settings.global_shortcut.clone();

        let (shortcuts, issues) = parse_configured_shortcuts(&settings);

        assert!(shortcuts.is_empty());
        assert_eq!(issues.len(), 2);
        assert!(issues
            .iter()
            .all(|issue| issue.error == "duplicate_shortcut"));
    }

    #[test]
    fn empty_shortcuts_have_no_parse_issues() {
        let mut settings = AppSettings::default().normalized();
        settings.global_shortcut.clear();
        settings.quick_paste_shortcut.clear();

        let (shortcuts, issues) = parse_configured_shortcuts(&settings);

        assert!(shortcuts.is_empty());
        assert!(issues.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn portal_shortcuts_follow_settings_keys_and_triggers() {
        use super::{portal, portal_shortcuts, GLOBAL_SHORTCUT_KEY, QUICK_PASTE_SHORTCUT_KEY};

        let settings = AppSettings::default().normalized();
        let (shortcuts, issues) = parse_configured_shortcuts(&settings);
        assert!(issues.is_empty());

        let mapped = portal_shortcuts(&shortcuts, "zh-CN");

        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[0].id, portal::TOGGLE_SHORTCUT_ID);
        assert_eq!(mapped[0].settings_key, GLOBAL_SHORTCUT_KEY);
        assert_eq!(mapped[0].trigger.as_deref(), Some("<Control><Shift>v"));
        assert_eq!(mapped[0].action, portal::ShortcutAction::TogglePanel);
        assert_eq!(mapped[1].id, portal::QUICK_PASTE_SHORTCUT_ID);
        assert_eq!(mapped[1].settings_key, QUICK_PASTE_SHORTCUT_KEY);
        assert_eq!(mapped[1].trigger.as_deref(), Some("<Control>grave"));
        assert_eq!(mapped[1].action, portal::ShortcutAction::QuickPaste);
    }
}
