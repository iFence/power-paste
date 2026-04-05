use anyhow::Result;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

pub(crate) fn set_launch_on_startup(app: &AppHandle, enabled: bool) -> Result<()> {
    let autostart = app.autolaunch();
    if enabled {
        autostart.enable()?;
    } else {
        autostart.disable()?;
    }
    Ok(())
}
