//! 文本消息的两种承载方式。
//!
//! 1. 协议原生文本消息：单个 `fileType` 为 `text`/`text/*` 且带 `preview` 的文件，
//!    接收端无需下载即可拿到文本，官方 LocalSend 客户端同样使用该方式。
//! 2. power-paste 文本包：以固定前缀命名的 txt 文件。旧版扫码页与旧版本用它承载文本，
//!    接收端仍按文本还原，保证新旧版本之间可以互传。

use std::collections::{HashMap, HashSet};

use localsend::model::transfer::FileDto;

// power-paste 文本包的文件名前缀。
pub(crate) const TEXT_FILE_PREFIX: &str = "PowerPaste-Text-";

// 文本包可还原为文本的实际上限，避免把超大 txt 塞进剪贴板。
pub(crate) const TEXT_RESTORE_MAX_BYTES: usize = 1024 * 1024;

// 判断文件名是否是 power-paste 文本包。
pub(crate) fn is_text_package(file_name: &str) -> bool {
    file_name.starts_with(TEXT_FILE_PREFIX) && file_name.to_ascii_lowercase().ends_with(".txt")
}

// 判断文件类型是否属于文本消息。
pub(crate) fn is_text_file_type(file_type: &str) -> bool {
    file_type == "text" || file_type.starts_with("text/")
}

// 判断是否为协议原生文本消息：单文件、文本类型且带 preview。
// 从混合请求中识别所有协议原生文本项，并返回文本与普通文件的 id 划分。
pub(crate) fn split_text_messages(
    files: &HashMap<String, FileDto>,
) -> (Vec<(String, String)>, HashSet<String>) {
    let mut messages = files
        .iter()
        .filter_map(|(id, file)| {
            if !is_text_file_type(&file.file_type) {
                return None;
            }
            let text = file
                .preview
                .clone()
                .filter(|value| !value.trim().is_empty())?;
            Some((id.clone(), text))
        })
        .collect::<Vec<_>>();
    messages.sort_by(|left, right| {
        let left_name = files
            .get(&left.0)
            .map(|file| file.file_name.as_str())
            .unwrap_or_default();
        let right_name = files
            .get(&right.0)
            .map(|file| file.file_name.as_str())
            .unwrap_or_default();
        left_name.cmp(right_name)
    });

    let text_ids = messages
        .iter()
        .map(|(id, _)| id.clone())
        .collect::<HashSet<_>>();
    let file_ids = files
        .keys()
        .filter(|id| !text_ids.contains(*id))
        .cloned()
        .collect::<HashSet<_>>();

    (messages, file_ids)
}

// 多个文本项在当前接收界面合并展示，写入剪贴板时按原顺序逐条记录。
pub(crate) fn joined_text_message(messages: &[(String, String)]) -> Option<String> {
    if messages.is_empty() {
        return None;
    }
    Some(
        messages
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n"),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::{
        is_text_file_type, is_text_package, joined_text_message, split_text_messages,
        TEXT_FILE_PREFIX,
    };
    use localsend::model::transfer::FileDto;

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
    fn splits_mixed_text_and_file_payloads() {
        let mut files = HashMap::new();
        files.insert(
            "file".into(),
            FileDto {
                id: "file".into(),
                file_name: "photo.png".into(),
                size: 10,
                file_type: "image/png".into(),
                sha256: None,
                preview: None,
                metadata: None,
            },
        );
        files.insert(
            "text".into(),
            FileDto {
                id: "text".into(),
                file_name: "message.txt".into(),
                size: 5,
                file_type: "text/plain".into(),
                sha256: None,
                preview: Some("hello".into()),
                metadata: None,
            },
        );

        let (messages, file_ids) = split_text_messages(&files);
        assert_eq!(messages, vec![("text".into(), "hello".into())]);
        assert_eq!(file_ids, HashSet::from(["file".to_string()]));
        assert_eq!(joined_text_message(&messages).as_deref(), Some("hello"));
    }
}
