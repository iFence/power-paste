//! 发送选择项：把文件、文件夹与剪贴板内容统一转化为可发送条目。

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use localsend::model::transfer::{FileDto, FileMetadata};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::{send, util};
use crate::models::AppError;

// 前端保存与回传的统一选择项；文件元数据由后端重新校验，不接受前端伪造的大小。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum LanSelectionItemDto {
    File {
        path: String,
        name: String,
        size: u64,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    Text {
        text: String,
    },
}

impl LanSelectionItemDto {
    pub(crate) fn text(text: String) -> Self {
        Self::Text { text }
    }

    fn display_name(&self, fallback: &str) -> String {
        match self {
            Self::File { name, .. } if !name.trim().is_empty() => {
                normalize_display_name(name, fallback)
            }
            _ => fallback.to_string(),
        }
    }
}

// 校验选择项并组装协议发送载荷；文件按当前文件系统元数据重新读取。
pub(super) fn collect_items(
    items: &[LanSelectionItemDto],
) -> Result<Vec<send::SendFile>, AppError> {
    let mut picked = Vec::new();

    for item in items {
        match item {
            LanSelectionItemDto::Text { text } => {
                let text = text.trim_end();
                if text.trim().is_empty() {
                    return Err(AppError::Message("empty_payload".into()));
                }
                picked.push(send::text_message(text));
            }
            LanSelectionItemDto::File { path, .. } => {
                let path = PathBuf::from(path);
                let metadata = fs::metadata(&path)
                    .map_err(|_| AppError::Message("lan_transfer_file_not_found".into()))?;
                if !metadata.is_file() {
                    return Err(AppError::Message("lan_transfer_file_not_found".into()));
                }

                let fallback = path
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or_else(|| "transfer-file".into());
                let file_name = item.display_name(&fallback);
                let id = uuid::Uuid::new_v4().to_string();
                picked.push(send::SendFile {
                    id: id.clone(),
                    file: FileDto {
                        id,
                        file_name,
                        size: metadata.len(),
                        file_type: util::infer_mime_type(&path),
                        sha256: None,
                        preview: None,
                        metadata: FileMetadata::from_fs_metadata(&metadata),
                    },
                    source: send::SendSource::Path(path),
                });
            }
        }
    }

    if picked.is_empty() {
        return Err(AppError::Message("lan_transfer_no_files".into()));
    }
    Ok(picked)
}

// 展开文件与文件夹选择；文件夹递归读取普通文件，不跟随符号链接。
pub(crate) fn inspect_selection(paths: Vec<String>) -> Result<Vec<LanSelectionItemDto>, AppError> {
    let mut selected = Vec::new();
    let mut seen = HashSet::new();

    for raw_path in paths {
        let path = PathBuf::from(raw_path);
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| AppError::Message("lan_selection_path_not_found".into()))?;
        let file_type = metadata.file_type();

        if file_type.is_symlink() {
            return Err(AppError::Message(
                "lan_selection_symlink_unsupported".into(),
            ));
        }
        if file_type.is_file() {
            push_file(&mut selected, &mut seen, &path, None)?;
            continue;
        }
        if file_type.is_dir() {
            let root_name = path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| "folder".into());
            collect_directory(&mut selected, &mut seen, &path, &path, &root_name)?;
            continue;
        }

        return Err(AppError::Message("lan_selection_path_unsupported".into()));
    }

    if selected.is_empty() {
        return Err(AppError::Message("lan_transfer_no_files".into()));
    }
    Ok(selected)
}

// 读取当前剪贴板；优先级与 LocalSend 保持一致：文本、图片、文件列表。
pub(crate) fn read_clipboard_selection(
    app: &AppHandle,
) -> Result<Vec<LanSelectionItemDto>, AppError> {
    let snapshot = crate::clipboard::plugin_reader::read_snapshot(app, true);

    if let Some(text) = snapshot.text.filter(|value| !value.trim().is_empty()) {
        return Ok(vec![LanSelectionItemDto::text(text)]);
    }

    if let Some(image) = snapshot.image {
        let path = write_clipboard_image(&image.png_bytes)?;
        let size = image.png_bytes.len() as u64;
        return Ok(vec![LanSelectionItemDto::File {
            path: path.to_string_lossy().to_string(),
            name: path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| "clipboard.png".into()),
            size,
            mime_type: "image/png".into(),
        }]);
    }

    if !snapshot.files.is_empty() {
        return inspect_selection(snapshot.files);
    }

    Err(AppError::Message("lan_transfer_clipboard_empty".into()))
}

fn push_file(
    selected: &mut Vec<LanSelectionItemDto>,
    seen: &mut HashSet<PathBuf>,
    path: &Path,
    display_name: Option<String>,
) -> Result<(), AppError> {
    let canonical = fs::canonicalize(path)
        .map_err(|_| AppError::Message("lan_selection_path_not_found".into()))?;
    if !seen.insert(canonical) {
        return Ok(());
    }

    let metadata = fs::metadata(path)
        .map_err(|_| AppError::Message("lan_selection_path_not_readable".into()))?;
    if !metadata.is_file() {
        return Err(AppError::Message("lan_selection_path_unsupported".into()));
    }

    let fallback = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "transfer-file".into());
    let name = display_name
        .map(|value| normalize_display_name(&value, &fallback))
        .unwrap_or(fallback);

    selected.push(LanSelectionItemDto::File {
        path: path.to_string_lossy().to_string(),
        name,
        size: metadata.len(),
        mime_type: util::infer_mime_type(path),
    });
    Ok(())
}

fn collect_directory(
    selected: &mut Vec<LanSelectionItemDto>,
    seen: &mut HashSet<PathBuf>,
    root: &Path,
    current: &Path,
    root_name: &str,
) -> Result<(), AppError> {
    let entries = fs::read_dir(current)
        .map_err(|_| AppError::Message("lan_selection_path_not_readable".into()))?;
    let mut paths = entries
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|_| AppError::Message("lan_selection_path_not_readable".into()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();

    for path in paths {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| AppError::Message("lan_selection_path_not_readable".into()))?;
        let file_type = metadata.file_type();

        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_directory(selected, seen, root, &path, root_name)?;
            continue;
        }
        if file_type.is_file() {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            let child_name = relative
                .components()
                .filter_map(|component| component.as_os_str().to_str())
                .collect::<Vec<_>>()
                .join("/");
            let display_name = if child_name.is_empty() {
                root_name.to_string()
            } else {
                format!("{root_name}/{child_name}")
            };
            push_file(selected, seen, &path, Some(display_name))?;
        }
    }

    Ok(())
}

fn normalize_display_name(value: &str, fallback: &str) -> String {
    let normalized = value
        .replace('\\', "/")
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != "." && *segment != "..")
        .map(util::sanitize_file_name)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("/");

    if normalized.is_empty() {
        util::sanitize_file_name(fallback)
    } else {
        normalized
    }
}

fn write_clipboard_image(bytes: &[u8]) -> Result<PathBuf, AppError> {
    let dir = std::env::temp_dir().join("power-paste-lan-paste");
    fs::create_dir_all(&dir)
        .map_err(|_| AppError::Message("lan_transfer_clipboard_write_failed".into()))?;
    let path = dir.join(format!("clipboard-{}.png", uuid::Uuid::new_v4()));
    fs::write(&path, bytes)
        .map_err(|_| AppError::Message("lan_transfer_clipboard_write_failed".into()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::{collect_items, inspect_selection, LanSelectionItemDto};

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("power-paste-{label}-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn expands_directories_with_relative_names_and_deduplicates() {
        let root = temp_dir("selection");
        let nested = root.join("nested");
        fs::create_dir_all(&nested).expect("create nested directory");
        let first = root.join("first.txt");
        let second = nested.join("second.bin");
        fs::write(&first, b"one").expect("write first");
        fs::write(&second, b"two").expect("write second");

        let items = inspect_selection(vec![
            root.to_string_lossy().to_string(),
            first.to_string_lossy().to_string(),
        ])
        .expect("inspect selection");

        assert_eq!(items.len(), 2);
        let names = items
            .iter()
            .map(|item| match item {
                LanSelectionItemDto::File { name, .. } => name.clone(),
                LanSelectionItemDto::Text { .. } => panic!("unexpected text item"),
            })
            .collect::<Vec<_>>();
        assert!(names.contains(&format!(
            "{}/first.txt",
            root.file_name().unwrap().to_string_lossy()
        )));
        assert!(names.contains(&format!(
            "{}/nested/second.bin",
            root.file_name().unwrap().to_string_lossy()
        )));

        fs::remove_dir_all(root).expect("cleanup temp directory");
    }

    #[test]
    fn rejects_missing_paths() {
        let error = inspect_selection(vec!["/path/that/does/not/exist".into()])
            .expect_err("missing path must fail");
        assert_eq!(error.to_string(), "lan_selection_path_not_found");
    }

    #[test]
    fn builds_file_and_text_payloads() {
        let root = temp_dir("selection-payload");
        fs::create_dir_all(&root).expect("create temp directory");
        let file = root.join("note.txt");
        fs::write(&file, b"hello").expect("write file");

        let items = vec![
            LanSelectionItemDto::File {
                path: file.to_string_lossy().to_string(),
                name: "docs/note.txt".into(),
                size: 0,
                mime_type: String::new(),
            },
            LanSelectionItemDto::text("message".into()),
        ];
        let picked = collect_items(&items).expect("build payload");

        assert_eq!(picked.len(), 2);
        assert_eq!(picked[0].file.file_name, "docs/note.txt");
        assert_eq!(picked[0].file.size, 5);
        assert_eq!(picked[1].file.preview.as_deref(), Some("message"));

        fs::remove_dir_all(root).expect("cleanup temp directory");
    }
}
