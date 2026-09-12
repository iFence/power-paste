//! 局域网互传的通用小工具：文件名清洗、唯一路径、MIME 推断与目录校验。

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use uuid::Uuid;

// 返回当前时间的毫秒时间戳，用于事件与文件名。
pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or_default()
}

// 清洗对端提供的文件名：去掉路径分隔符与非法字符，避免越权写盘。
pub(crate) fn sanitize_file_name(value: &str) -> String {
    let name = value
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("transfer-file")
        .trim();
    let sanitized = name
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .trim()
        .to_string();

    if sanitized.is_empty() {
        "transfer-file".into()
    } else {
        sanitized.chars().take(180).collect()
    }
}

// 在目录内生成不冲突的文件路径，重名时追加序号。
pub(crate) fn unique_file_path(dir: &Path, file_name: &str) -> PathBuf {
    let candidate = dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }

    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("transfer-file");
    let extension = path.extension().and_then(|value| value.to_str());

    for index in 1..1000 {
        let next_name = match extension {
            Some(extension) if !extension.is_empty() => format!("{stem} ({index}).{extension}"),
            _ => format!("{stem} ({index})"),
        };
        let next = dir.join(next_name);
        if !next.exists() {
            return next;
        }
    }

    dir.join(format!("{stem}-{}", Uuid::new_v4()))
}

// 根据扩展名推断 MIME，未知类型回退为二进制流。
pub(crate) fn infer_mime_type(path: &Path) -> String {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "txt" | "log" | "md" => "text/plain",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .into()
}

// 校验接收目录可用：必须存在、是目录且可写。
pub(crate) fn validate_download_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("lan_transfer_download_dir_missing");
    }
    if !path.is_dir() {
        anyhow::bail!("lan_transfer_download_dir_not_directory");
    }
    let probe = path.join(format!(".power-paste-write-test-{}", Uuid::new_v4()));
    fs::write(&probe, b"ok").context("lan_transfer_download_dir_not_writable")?;
    fs::remove_file(&probe).context("lan_transfer_download_dir_cleanup_failed")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{infer_mime_type, sanitize_file_name, unique_file_path};

    #[test]
    fn sanitizes_file_names() {
        assert_eq!(sanitize_file_name("../a:b?.txt"), "a_b_.txt");
        assert_eq!(sanitize_file_name("..."), "transfer-file");
        assert_eq!(sanitize_file_name("dir/photo.png"), "photo.png");
    }

    #[test]
    fn keeps_unique_file_name_when_available() {
        let dir = std::env::temp_dir();
        let path = unique_file_path(&dir, "power-paste-unique-name-test.txt");
        assert!(path.ends_with("power-paste-unique-name-test.txt"));
    }

    #[test]
    fn infers_common_mime_types() {
        assert_eq!(infer_mime_type(Path::new("a/b/c.png")), "image/png");
        assert_eq!(infer_mime_type(Path::new("notes.txt")), "text/plain");
        assert_eq!(
            infer_mime_type(Path::new("data.bin")),
            "application/octet-stream"
        );
    }
}
