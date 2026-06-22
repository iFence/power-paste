use anyhow::Result;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

use crate::clipboard::launch_on_startup_supported;

pub(crate) const BACKGROUND_STARTUP_ARG: &str = "--background-startup";

pub(crate) fn is_background_startup_args<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .any(|arg| arg.as_ref() == BACKGROUND_STARTUP_ARG)
}

fn should_apply_launch_on_startup(current_enabled: Option<bool>, target_enabled: bool) -> bool {
    match current_enabled {
        Some(current_enabled) => target_enabled || current_enabled != target_enabled,
        None => target_enabled,
    }
}

pub(crate) fn set_launch_on_startup(app: &AppHandle, enabled: bool) -> Result<()> {
    if !launch_on_startup_supported() {
        if enabled {
            anyhow::bail!("unsupported_launch_on_startup");
        }
        return Ok(());
    }

    let autostart = app.autolaunch();
    let current_enabled = autostart.is_enabled().ok();

    if !should_apply_launch_on_startup(current_enabled, enabled) {
        return Ok(());
    }

    if enabled {
        let _ = autostart.disable();
        autostart.enable()?;
    } else {
        autostart.disable()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_background_startup_args, should_apply_launch_on_startup};

    #[test]
    fn detects_background_startup_arg() {
        assert!(is_background_startup_args(["--background-startup"]));
    }

    #[test]
    fn detects_background_startup_arg_among_other_args() {
        assert!(is_background_startup_args([
            "--flag",
            "--background-startup",
            "--other",
        ]));
    }

    #[test]
    fn ignores_regular_startup_args() {
        assert!(!is_background_startup_args(["--flag", "--other"]));
    }

    #[test]
    fn enables_when_state_is_unknown_but_target_is_enabled() {
        assert!(should_apply_launch_on_startup(None, true));
    }

    #[test]
    fn skips_disable_when_state_is_unknown_and_target_is_disabled() {
        assert!(!should_apply_launch_on_startup(None, false));
    }

    #[test]
    fn skips_transition_when_state_already_matches_target() {
        assert!(!should_apply_launch_on_startup(Some(false), false));
    }

    #[test]
    fn reapplies_when_target_is_enabled_to_refresh_arguments() {
        assert!(should_apply_launch_on_startup(Some(true), true));
    }

    #[test]
    fn applies_transition_when_state_differs_from_target() {
        assert!(should_apply_launch_on_startup(Some(true), false));
        assert!(should_apply_launch_on_startup(Some(false), true));
    }
}
