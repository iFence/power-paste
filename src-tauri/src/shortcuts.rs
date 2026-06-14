use anyhow::Result;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

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

pub(crate) fn unregister_configured_shortcuts(app: &AppHandle, settings: &AppSettings) {
    let (shortcuts, _) = parse_configured_shortcuts(settings);
    for shortcut in shortcuts {
        let _ = app.global_shortcut().unregister(shortcut.shortcut);
    }
}

pub(crate) fn register_shortcuts_nonfatal(
    app: &AppHandle,
    settings: &AppSettings,
) -> ShortcutStatusDto {
    let (shortcuts, mut issues) = parse_configured_shortcuts(settings);
    let mut status = ShortcutStatusDto {
        issues: Vec::new(),
        ..ShortcutStatusDto::default()
    };

    for shortcut in shortcuts {
        match app.global_shortcut().register(shortcut.shortcut) {
            Ok(()) => {
                if shortcut.key == GLOBAL_SHORTCUT_KEY {
                    status.global_shortcut_registered = true;
                } else if shortcut.key == QUICK_PASTE_SHORTCUT_KEY {
                    status.quick_paste_shortcut_registered = true;
                }
            }
            Err(error) => issues.push(ShortcutIssueDto {
                key: shortcut.key.into(),
                shortcut: shortcut.value,
                error: error.to_string(),
            }),
        }
    }

    status.issues = issues;
    status
}

pub(crate) fn register_shortcuts_strict(
    app: &AppHandle,
    settings: &AppSettings,
) -> Result<ShortcutStatusDto> {
    let (shortcuts, issues) = parse_configured_shortcuts(settings);
    if let Some(issue) = issues.first() {
        anyhow::bail!("{}", issue.error);
    }

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
        if shortcut.key == GLOBAL_SHORTCUT_KEY {
            status.global_shortcut_registered = true;
        } else if shortcut.key == QUICK_PASTE_SHORTCUT_KEY {
            status.quick_paste_shortcut_registered = true;
        }
    }

    Ok(status)
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
}
