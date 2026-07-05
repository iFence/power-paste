use std::{collections::HashSet, path::PathBuf};

use anyhow::Result;

use crate::{
    history::source_app_icon_data_url,
    models::{ForegroundAppResult, InstalledAppDto},
};

pub(crate) fn list_installed_apps() -> Result<Vec<InstalledAppDto>> {
    #[cfg(windows)]
    {
        return list_windows_installed_apps();
    }
    #[cfg(target_os = "macos")]
    {
        return list_macos_installed_apps();
    }
    #[cfg(target_os = "linux")]
    {
        return Ok(Vec::new());
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        Ok(Vec::new())
    }
}

fn build_installed_app(
    platform: &str,
    display_name: String,
    process_name: String,
    app_path: Option<String>,
    bundle_id: Option<String>,
) -> InstalledAppDto {
    let icon_data_url = source_app_icon_data_url(&ForegroundAppResult {
        process_name: process_name.clone(),
        display_name: display_name.clone(),
        icon_png_base64: None,
        app_path: app_path.clone(),
        bundle_id: bundle_id.clone(),
    });

    InstalledAppDto {
        platform: platform.into(),
        display_name,
        process_name,
        app_path,
        bundle_id,
        icon_data_url,
    }
}

fn sort_and_dedupe_apps(mut apps: Vec<InstalledAppDto>) -> Vec<InstalledAppDto> {
    let mut seen = HashSet::new();
    apps.retain(|app| {
        let key = app
            .app_path
            .as_deref()
            .or(app.bundle_id.as_deref())
            .unwrap_or(app.process_name.as_str())
            .trim()
            .replace('\\', "/")
            .to_ascii_lowercase();
        !key.is_empty() && seen.insert(key)
    });
    apps.sort_by(|left, right| {
        left.display_name
            .to_ascii_lowercase()
            .cmp(&right.display_name.to_ascii_lowercase())
    });
    apps
}

#[cfg(windows)]
fn list_windows_installed_apps() -> Result<Vec<InstalledAppDto>> {
    use std::ffi::c_void;
    use windows_sys::Win32::{
        Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS},
        System::Registry::{
            RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
            HKEY_LOCAL_MACHINE, KEY_READ, RRF_RT_REG_DWORD, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ,
        },
    };

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn read_string(key: HKEY, name: &str) -> Option<String> {
        let name = wide(name);
        let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ;
        let mut data_len = 0u32;
        let status = unsafe {
            RegGetValueW(
                key,
                std::ptr::null(),
                name.as_ptr(),
                flags,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut data_len,
            )
        };
        if status != ERROR_SUCCESS || data_len < 2 {
            return None;
        }

        let mut buffer = vec![0u16; (data_len as usize + 1) / 2];
        let status = unsafe {
            RegGetValueW(
                key,
                std::ptr::null(),
                name.as_ptr(),
                flags,
                std::ptr::null_mut(),
                buffer.as_mut_ptr() as *mut c_void,
                &mut data_len,
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }

        let end = buffer
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(buffer.len());
        let value = String::from_utf16_lossy(&buffer[..end]).trim().to_string();
        (!value.is_empty()).then_some(value)
    }

    fn read_dword(key: HKEY, name: &str) -> Option<u32> {
        let name = wide(name);
        let mut value = 0u32;
        let mut data_len = std::mem::size_of::<u32>() as u32;
        let status = unsafe {
            RegGetValueW(
                key,
                std::ptr::null(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                &mut value as *mut u32 as *mut c_void,
                &mut data_len,
            )
        };
        (status == ERROR_SUCCESS).then_some(value)
    }

    fn display_icon_path(value: &str) -> Option<PathBuf> {
        let trimmed = value.trim();
        let candidate = if let Some(rest) = trimmed.strip_prefix('"') {
            rest.split_once('"').map(|(path, _)| path).unwrap_or(rest)
        } else {
            trimmed
                .split_once(',')
                .map(|(path, _)| path)
                .unwrap_or(trimmed)
        }
        .trim();

        let path = PathBuf::from(candidate);
        let is_exe = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);
        (is_exe && path.is_file()).then_some(path)
    }

    fn install_location_exe(value: &str) -> Option<PathBuf> {
        let dir = PathBuf::from(value.trim());
        if !dir.is_dir() {
            return None;
        }

        std::fs::read_dir(dir)
            .ok()?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .find(|path| {
                path.is_file()
                    && path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext.eq_ignore_ascii_case("exe"))
                        .unwrap_or(false)
            })
    }

    fn resolve_app_path(key: HKEY) -> Option<PathBuf> {
        read_string(key, "DisplayIcon")
            .as_deref()
            .and_then(display_icon_path)
            .or_else(|| {
                read_string(key, "InstallLocation")
                    .as_deref()
                    .and_then(install_location_exe)
            })
    }

    fn enumerate_root(root: HKEY, subkey_path: &str, apps: &mut Vec<InstalledAppDto>) {
        let mut root_key: HKEY = std::ptr::null_mut();
        let subkey_path_w = wide(subkey_path);
        let opened =
            unsafe { RegOpenKeyExW(root, subkey_path_w.as_ptr(), 0, KEY_READ, &mut root_key) };
        if opened != ERROR_SUCCESS {
            return;
        }

        let mut index = 0u32;
        loop {
            let mut name_buffer = vec![0u16; 260];
            let mut name_len = name_buffer.len() as u32;
            let status = unsafe {
                RegEnumKeyExW(
                    root_key,
                    index,
                    name_buffer.as_mut_ptr(),
                    &mut name_len,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if status == ERROR_NO_MORE_ITEMS {
                break;
            }
            if status != ERROR_SUCCESS {
                index += 1;
                continue;
            }

            let name = String::from_utf16_lossy(&name_buffer[..name_len as usize]);
            let mut app_key: HKEY = std::ptr::null_mut();
            let name_w = wide(&name);
            let opened =
                unsafe { RegOpenKeyExW(root_key, name_w.as_ptr(), 0, KEY_READ, &mut app_key) };
            if opened == ERROR_SUCCESS {
                if read_dword(app_key, "SystemComponent") != Some(1) {
                    if let Some(display_name) = read_string(app_key, "DisplayName") {
                        if let Some(app_path) = resolve_app_path(app_key) {
                            let process_name = app_path
                                .file_stem()
                                .and_then(|value| value.to_str())
                                .unwrap_or(display_name.as_str())
                                .to_string();
                            apps.push(build_installed_app(
                                "windows",
                                display_name,
                                process_name,
                                Some(app_path.to_string_lossy().to_string()),
                                None,
                            ));
                        }
                    }
                }
                unsafe {
                    RegCloseKey(app_key);
                }
            }

            index += 1;
        }

        unsafe {
            RegCloseKey(root_key);
        }
    }

    let mut apps = Vec::new();
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        enumerate_root(
            root,
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            &mut apps,
        );
        enumerate_root(
            root,
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
            &mut apps,
        );
    }

    Ok(sort_and_dedupe_apps(apps))
}

#[cfg(target_os = "macos")]
fn list_macos_installed_apps() -> Result<Vec<InstalledAppDto>> {
    fn plutil_raw(plist: &std::path::Path, key: &str) -> Option<String> {
        let output = std::process::Command::new("plutil")
            .arg("-extract")
            .arg(key)
            .arg("raw")
            .arg("-o")
            .arg("-")
            .arg(plist)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
        (!value.is_empty()).then_some(value)
    }

    fn scan_dir(dir: PathBuf, apps: &mut Vec<InstalledAppDto>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };

        for path in entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
        {
            let is_app = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("app"))
                .unwrap_or(false);
            if !is_app {
                continue;
            }

            let plist = path.join("Contents").join("Info.plist");
            if !plist.is_file() {
                continue;
            }

            let fallback_name = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string();
            let display_name = plutil_raw(&plist, "CFBundleDisplayName")
                .or_else(|| plutil_raw(&plist, "CFBundleName"))
                .unwrap_or_else(|| fallback_name.clone());
            if display_name.trim().is_empty() {
                continue;
            }
            let process_name =
                plutil_raw(&plist, "CFBundleExecutable").unwrap_or_else(|| fallback_name.clone());
            let bundle_id = plutil_raw(&plist, "CFBundleIdentifier");

            apps.push(build_installed_app(
                "macos",
                display_name,
                process_name,
                Some(path.to_string_lossy().to_string()),
                bundle_id,
            ));
        }
    }

    let mut apps = Vec::new();
    scan_dir(PathBuf::from("/Applications"), &mut apps);
    scan_dir(PathBuf::from("/System/Applications"), &mut apps);
    if let Some(home) = std::env::var_os("HOME") {
        scan_dir(PathBuf::from(home).join("Applications"), &mut apps);
    }

    Ok(sort_and_dedupe_apps(apps))
}
