use anyhow::Result;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

use crate::clipboard::launch_on_startup_supported;

pub(crate) fn set_launch_on_startup(app: &AppHandle, enabled: bool) -> Result<()> {
    if !launch_on_startup_supported() {
        if enabled {
            anyhow::bail!("unsupported_launch_on_startup");
        }
        return Ok(());
    }

    let autostart = app.autolaunch();
    if enabled {
        autostart.enable()?;
    } else {
        autostart.disable()?;
    }
    Ok(())
}
