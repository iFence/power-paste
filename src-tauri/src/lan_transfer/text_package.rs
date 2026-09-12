//! 文本消息的两种承载方式。
//!
//! 1. 协议原生文本消息：单个 `fileType` 为 `text`/`text/*` 且带 `preview` 的文件，
//!    接收端无需下载即可拿到文本，官方 LocalSend 客户端同样使用该方式。
//! 2. power-paste 文本包：以固定前缀命名的 txt 文件，浏览器页面与旧版本使用它承载文本。

use localsend::model::transfer::FileDto;

// power-paste 文本包的文件名前缀。
pub(crate) const TEXT_FILE_PREFIX: &str = "PowerPaste-Text-";

// 文本包可还原为文本的实际上限，避免把超大 txt 塞进剪贴板。
pub(crate) const TEXT_RESTORE_MAX_BYTES: usize = 1024 * 1024;

// 判断文件名是否是 power-paste 文本包。
pub(crate) fn is_text_package(file_name: &str) -> bool {
    file_name.starts_with(TEXT_FILE_PREFIX)
        && file_name
            .to_ascii_lowercase()
            .ends_with(".txt")
}

// 判断文件类型是否属于文本消息。
pub(crate) fn is_text_file_type(file_type: &str) -> bool {
    file_type == "text" || file_type.starts_with("text/")
}

// 判断是否为协议原生文本消息：单文件、文本类型且带 preview。
pub(crate) fn protocol_text_message(files: &[&FileDto]) -> Option<String> {
    if files.len() != 1 {
        return None;
    }
    let file = files[0];
    if !is_text_file_type(&file.file_type) {
        return None;
    }
    file.preview
        .clone()
        .filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::{is_text_file_type, is_text_package, protocol_text_message, TEXT_FILE_PREFIX};
    use localsend::model::transfer::FileDto;

    fn file(file_type: &str, preview: Option<&str>) -> FileDto {
        FileDto {
            id: "1".into(),
            file_name: "message.txt".into(),
            size: 3,
            file_type: file_type.into(),
            sha256: None,
            preview: preview.map(ToString::to_string),
            metadata: None,
        }
    }

    #[test]
    fn builds_and_detects_text_package_names() {
        let name = format!("{TEXT_FILE_PREFIX}1731000000.txt");
        assert_eq!(name, "PowerPaste-Text-1731000000.txt");
        assert!(is_text_package(&name));
        assert!(!is_text_package("PowerPaste-Text-1.pdf"));
        assert!(!is_text_package("Other-Text-1.txt"));
    }

    #[test]
    fn recognises_text_file_types() {
        assert!(is_text_file_type("text"));
        assert!(is_text_file_type("text/plain"));
        assert!(!is_text_file_type("application/pdf"));
    }

    #[test]
    fn detects_protocol_text_messages_only_for_single_previewed_text_files() {
        let message = file("text", Some("hello"));
        assert_eq!(
            protocol_text_message(&[&message]).as_deref(),
            Some("hello")
        );

        let without_preview = file("text/plain", None);
        assert!(protocol_text_message(&[&without_preview]).is_none());

        let binary = file("application/pdf", Some("hello"));
        assert!(protocol_text_message(&[&binary]).is_none());

        let plain = file("text/plain", Some("hi"));
        assert!(protocol_text_message(&[&plain, &plain]).is_none());
    }
}
