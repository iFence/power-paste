use anyhow::{Context, Result};

use crate::models::{WINDOWS_RUN_KEY, WINDOWS_RUN_VALUE_NAME};

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
    if enabled {
        anyhow::bail!("unsupported_launch_on_startup");
    }
    Ok(())
}
