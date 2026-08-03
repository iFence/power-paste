#[cfg(target_os = "macos")]
use std::{ffi::OsStr, fs, os::unix::fs::MetadataExt, path::Path, process::Command};

#[cfg(target_os = "macos")]
use anyhow::Context;
use anyhow::Result;
use tauri::AppHandle;
#[cfg(target_os = "macos")]
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

use crate::clipboard::launch_on_startup_supported;

pub(crate) const BACKGROUND_STARTUP_ARG: &str = "--background-startup";

#[cfg(target_os = "macos")]
const LEGACY_MACOS_LAUNCH_AGENT_LABEL: &str = "com.yulei.powerpaste";

#[cfg(target_os = "macos")]
fn legacy_macos_launch_agent_path(home_dir: &Path) -> std::path::PathBuf {
    home_dir
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{LEGACY_MACOS_LAUNCH_AGENT_LABEL}.plist"))
}

#[cfg(target_os = "macos")]
fn should_unload_legacy_macos_launch_agent(current_service_name: Option<&OsStr>) -> bool {
    current_service_name != Some(OsStr::new(LEGACY_MACOS_LAUNCH_AGENT_LABEL))
}

#[cfg(target_os = "macos")]
fn cleanup_legacy_macos_launch_agent_at(
    launch_agent_path: &Path,
    current_service_name: Option<&OsStr>,
) -> Result<()> {
    if !launch_agent_path.exists() {
        return Ok(());
    }

    if should_unload_legacy_macos_launch_agent(current_service_name) {
        let uid = fs::metadata(launch_agent_path)
            .context("failed to inspect legacy macOS launch agent")?
            .uid();
        let service_target = format!("gui/{uid}/{LEGACY_MACOS_LAUNCH_AGENT_LABEL}");
        let print_output = Command::new("launchctl")
            .args(["print", &service_target])
            .output()
            .context("failed to inspect legacy macOS launch agent service")?;

        if print_output.status.success() {
            let bootout_output = Command::new("launchctl")
                .args(["bootout", &service_target])
                .output()
                .context("failed to unload legacy macOS launch agent service")?;
            if !bootout_output.status.success() {
                anyhow::bail!(
                    "failed to unload legacy macOS launch agent service: {}",
                    String::from_utf8_lossy(&bootout_output.stderr).trim()
                );
            }
        }
    }

    fs::remove_file(launch_agent_path).context("failed to remove legacy macOS launch agent")?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn cleanup_legacy_macos_launch_agent(app: &AppHandle) -> Result<()> {
    let home_dir = app.path().home_dir()?;
    let launch_agent_path = legacy_macos_launch_agent_path(&home_dir);
    let current_service_name = std::env::var_os("XPC_SERVICE_NAME");
    cleanup_legacy_macos_launch_agent_at(&launch_agent_path, current_service_name.as_deref())
}

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
    #[cfg(target_os = "macos")]
    cleanup_legacy_macos_launch_agent(app)?;

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

    #[cfg(target_os = "macos")]
    use super::{
        cleanup_legacy_macos_launch_agent_at, legacy_macos_launch_agent_path,
        should_unload_legacy_macos_launch_agent, LEGACY_MACOS_LAUNCH_AGENT_LABEL,
    };
    #[cfg(target_os = "macos")]
    use anyhow::Context;
    #[cfg(target_os = "macos")]
    use std::{ffi::OsStr, fs};

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

    #[cfg(target_os = "macos")]
    #[test]
    fn resolves_legacy_macos_launch_agent_path() {
        let path = legacy_macos_launch_agent_path(std::path::Path::new("/Users/tester"));

        assert_eq!(
            path,
            std::path::PathBuf::from(
                "/Users/tester/Library/LaunchAgents/com.yulei.powerpaste.plist"
            )
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn skips_unloading_when_running_as_legacy_service() {
        assert!(!should_unload_legacy_macos_launch_agent(Some(OsStr::new(
            LEGACY_MACOS_LAUNCH_AGENT_LABEL
        ))));
        assert!(should_unload_legacy_macos_launch_agent(Some(OsStr::new(
            "Power Paste"
        ))));
        assert!(should_unload_legacy_macos_launch_agent(None));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn removes_legacy_macos_launch_agent_idempotently() -> anyhow::Result<()> {
        let temp_root =
            std::env::temp_dir().join(format!("power-paste-startup-test-{}", uuid::Uuid::new_v4()));
        let launch_agent_path = legacy_macos_launch_agent_path(&temp_root);
        let parent = launch_agent_path
            .parent()
            .context("legacy launch agent test path has no parent")?;
        fs::create_dir_all(parent)?;
        fs::write(&launch_agent_path, "test")?;

        let legacy_service_name = OsStr::new(LEGACY_MACOS_LAUNCH_AGENT_LABEL);
        cleanup_legacy_macos_launch_agent_at(&launch_agent_path, Some(legacy_service_name))?;
        assert!(!launch_agent_path.exists());
        cleanup_legacy_macos_launch_agent_at(&launch_agent_path, Some(legacy_service_name))?;

        fs::remove_dir_all(temp_root)?;
        Ok(())
    }
}
