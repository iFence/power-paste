use anyhow::{Context, Result};

#[cfg(windows)]
use crate::models::{WINDOWS_RUN_KEY, WINDOWS_RUN_VALUE_NAME};

#[cfg(target_os = "macos")]
use std::{fs, path::PathBuf, process::Command};

#[cfg(windows)]
fn quote_ps_single(text: &str) -> String {
    text.replace('\'', "''")
}

#[cfg(windows)]
// Startup registration is managed via the current user's Run key so no admin rights are required.
pub(crate) fn set_launch_on_startup(enabled: bool) -> Result<()> {
    if enabled {
        let exe_path = std::env::current_exe().context("failed to locate current executable")?;
        let exe_path = quote_ps_single(&exe_path.to_string_lossy());
        crate::powershell(&format!(
            "$ErrorActionPreference='Stop'; \
             New-Item -Path '{WINDOWS_RUN_KEY}' -Force | Out-Null; \
             Set-ItemProperty -Path '{WINDOWS_RUN_KEY}' -Name '{WINDOWS_RUN_VALUE_NAME}' -Value '\"{exe_path}\"'"
        ))?;
    } else {
        crate::powershell(&format!(
            "$ErrorActionPreference='Stop'; \
             if (Test-Path '{WINDOWS_RUN_KEY}') {{ \
               Remove-ItemProperty -Path '{WINDOWS_RUN_KEY}' -Name '{WINDOWS_RUN_VALUE_NAME}' -ErrorAction SilentlyContinue \
             }}"
        ))?;
    }

    Ok(())
}

#[cfg(not(windows))]
pub(crate) fn set_launch_on_startup(enabled: bool) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        const MACOS_LAUNCH_AGENT_LABEL: &str = "com.yulei.powerpaste";

        fn launch_agent_path() -> Result<PathBuf> {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .context("failed to resolve HOME for launch agent")?;
            Ok(home
                .join("Library")
                .join("LaunchAgents")
                .join(format!("{MACOS_LAUNCH_AGENT_LABEL}.plist")))
        }

        fn launchctl(args: &[&str]) -> Result<()> {
            let status = Command::new("launchctl").args(args).status()?;
            if status.success() {
                Ok(())
            } else {
                anyhow::bail!("launchctl {} failed", args.join(" "))
            }
        }

        fn launchctl_domain() -> Result<String> {
            let output = Command::new("id").arg("-u").output()?;
            if !output.status.success() {
                anyhow::bail!("failed to resolve current uid for launchctl domain");
            }
            let uid = String::from_utf8(output.stdout)?.trim().to_string();
            if uid.is_empty() {
                anyhow::bail!("empty uid for launchctl domain");
            }
            Ok(format!("gui/{uid}"))
        }

        let plist_path = launch_agent_path()?;
        let domain = launchctl_domain()?;

        if enabled {
            let exe_path =
                std::env::current_exe().context("failed to locate current executable")?;
            let exe_path_xml = exe_path
                .to_string_lossy()
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");

            if let Some(parent) = plist_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let plist = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{MACOS_LAUNCH_AGENT_LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe_path_xml}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
</dict>
</plist>
"#
            );

            fs::write(&plist_path, plist)?;

            let plist_path_string = plist_path.to_string_lossy().to_string();
            let _ = launchctl(&["bootout", &domain, &plist_path_string]);
            launchctl(&["bootstrap", &domain, &plist_path_string])?;
            return Ok(());
        }

        if plist_path.exists() {
            let plist_path_string = plist_path.to_string_lossy().to_string();
            let _ = launchctl(&["bootout", &domain, &plist_path_string]);
            fs::remove_file(&plist_path)?;
        }

        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    if enabled {
        anyhow::bail!("unsupported_launch_on_startup");
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}
