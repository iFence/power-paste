use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::{
    models::{AppSettings, CapturedClipboard, StoragePaths, StoredClipboardItem},
    rich_text::normalize_rich_text_payload,
    storage::{image_preview_png_from_bytes, preview_text, sha256_hex},
};

pub(crate) struct SqliteHistoryStore {
    connection: Connection,
}

pub(crate) struct UpsertedCapture {
    pub(crate) inserted: bool,
    pub(crate) item: StoredClipboardItem,
}

fn ensure_column(connection: &Connection, name: &str, sql_type: &str) -> Result<()> {
    let mut statement = connection.prepare("PRAGMA table_info(clipboard_items)")?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for column in columns {
        if column? == name {
            return Ok(());
        }
    }

    connection.execute(
        &format!("ALTER TABLE clipboard_items ADD COLUMN {name} {sql_type}"),
        [],
    )?;
    Ok(())
}

impl SqliteHistoryStore {
    pub(crate) fn new(paths: &StoragePaths) -> Result<Self> {
        let connection = Connection::open(&paths.db_path)
            .with_context(|| format!("failed to open sqlite db at {}", paths.db_path.display()))?;
        let store = Self { connection };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        self.connection.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS clipboard_items (
              id TEXT PRIMARY KEY NOT NULL,
              kind TEXT NOT NULL,
              created_at TEXT NOT NULL,
              pinned_at TEXT,
              preview TEXT NOT NULL,
              full_text TEXT,
              html_text TEXT,
              rtf_text TEXT,
              image_png BLOB,
              image_original_bytes BLOB,
              image_original_mime TEXT,
              image_preview_png BLOB,
              image_width INTEGER,
              image_height INTEGER,
              source_app TEXT,
              source_icon_data_url TEXT,
              hash TEXT NOT NULL,
              pinned INTEGER NOT NULL DEFAULT 0,
              favorite INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_clipboard_items_sort
              ON clipboard_items (pinned DESC, pinned_at DESC, favorite DESC, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_clipboard_items_hash
              ON clipboard_items (hash);
            CREATE INDEX IF NOT EXISTS idx_clipboard_items_kind_full_text
              ON clipboard_items (kind, full_text);
            "#,
        )?;
        ensure_column(&self.connection, "image_original_bytes", "BLOB")?;
        ensure_column(&self.connection, "image_original_mime", "TEXT")?;
        ensure_column(&self.connection, "image_preview_png", "BLOB")?;
        Ok(())
    }

    pub(crate) fn list_all(&self) -> Result<Vec<StoredClipboardItem>> {
        self.query_items(None, None, 0)
    }

    pub(crate) fn list_history(
        &self,
        query: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<StoredClipboardItem>> {
        self.query_items(query, Some(limit), offset)
    }

    pub(crate) fn count_history(&self, query: Option<&str>) -> Result<usize> {
        let query = query.unwrap_or("").trim().to_lowercase();
        if query.is_empty() {
            return self
                .connection
                .query_row("SELECT COUNT(*) FROM clipboard_items", [], |row| {
                    row.get::<_, i64>(0)
                })
                .map(|count| count.max(0) as usize)
                .map_err(Into::into);
        }

        self.connection
            .query_row(
                r#"
                SELECT COUNT(*)
                FROM clipboard_items
                WHERE lower(
                  preview || char(10) || coalesce(full_text, '') || char(10) || coalesce(source_app, '')
                ) LIKE '%' || ?1 || '%'
                "#,
                params![query],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count.max(0) as usize)
            .map_err(Into::into)
    }

    fn query_items(
        &self,
        query: Option<&str>,
        limit: Option<usize>,
        offset: usize,
    ) -> Result<Vec<StoredClipboardItem>> {
        let limit = limit.unwrap_or(i64::MAX as usize).min(i64::MAX as usize) as i64;
        let offset = offset.min(i64::MAX as usize) as i64;
        let query = query.unwrap_or("").trim().to_lowercase();
        let sql = if query.is_empty() {
            r#"
            SELECT id, kind, created_at, pinned_at, preview, full_text, html_text, rtf_text,
                   image_png, image_original_bytes, image_original_mime,
                   image_preview_png, image_width, image_height, source_app, source_icon_data_url,
                   hash, pinned, favorite
            FROM clipboard_items
            ORDER BY pinned DESC, pinned_at DESC, favorite DESC, created_at DESC
            LIMIT ?1 OFFSET ?2
            "#
        } else {
            r#"
            SELECT id, kind, created_at, pinned_at, preview, full_text, html_text, rtf_text,
                   image_png, image_original_bytes, image_original_mime,
                   image_preview_png, image_width, image_height, source_app, source_icon_data_url,
                   hash, pinned, favorite
            FROM clipboard_items
            WHERE lower(
              preview || char(10) || coalesce(full_text, '') || char(10) || coalesce(source_app, '')
            ) LIKE '%' || ?1 || '%'
            ORDER BY pinned DESC, pinned_at DESC, favorite DESC, created_at DESC
            LIMIT ?2 OFFSET ?3
            "#
        };

        let mut statement = self.connection.prepare(sql)?;
        let rows = if query.is_empty() {
            statement.query_map(params![limit, offset], Self::row_to_item)?
        } else {
            statement.query_map(params![query, limit, offset], Self::row_to_item)?
        };

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn get_item(&self, id: &str) -> Result<Option<StoredClipboardItem>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT id, kind, created_at, pinned_at, preview, full_text, html_text, rtf_text,
                   image_png, image_original_bytes, image_original_mime,
                   image_preview_png, image_width, image_height, source_app, source_icon_data_url,
                   hash, pinned, favorite
            FROM clipboard_items
            WHERE id = ?1
            "#,
        )?;
        statement
            .query_row(params![id], Self::row_to_item)
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn upsert_capture(
        &mut self,
        capture: CapturedClipboard,
        source_app: Option<(String, Option<String>)>,
        settings: &AppSettings,
    ) -> Result<UpsertedCapture> {
        let tx = self.connection.transaction()?;
        let now = Utc::now().to_rfc3339();
        let hash = capture_hash(&capture).to_string();
        let matching_text = capture_matching_text(&capture);

        let existing = if let Some(text) = matching_text.as_deref() {
            tx.prepare(
                r#"
                SELECT id, kind, created_at, pinned_at, preview, full_text, html_text, rtf_text,
                       image_png, image_original_bytes, image_original_mime,
                       image_preview_png, image_width, image_height, source_app, source_icon_data_url,
                       hash, pinned, favorite
                FROM clipboard_items
                WHERE hash = ?1 OR (kind = 'text' AND full_text = ?2)
                LIMIT 1
                "#,
            )?
            .query_row(params![hash, text], Self::row_to_item)
            .optional()?
        } else {
            tx.prepare(
                r#"
                SELECT id, kind, created_at, pinned_at, preview, full_text, html_text, rtf_text,
                       image_png, image_original_bytes, image_original_mime,
                       image_preview_png, image_width, image_height, source_app, source_icon_data_url,
                       hash, pinned, favorite
                FROM clipboard_items
                WHERE hash = ?1
                LIMIT 1
                "#,
            )?
            .query_row(params![hash], Self::row_to_item)
            .optional()?
        };

        let (source_app_name, source_icon_data_url) = match source_app {
            Some((name, icon)) => (Some(name), icon),
            None => (None, None),
        };

        match existing {
            Some(existing) => {
                let next = apply_capture(
                    existing,
                    capture,
                    &now,
                    source_app_name,
                    source_icon_data_url,
                );
                tx.execute(
                    r#"
                    UPDATE clipboard_items
                    SET kind = ?2,
                        created_at = ?3,
                        pinned_at = ?4,
                        preview = ?5,
                        full_text = ?6,
                        html_text = ?7,
                        rtf_text = ?8,
                        image_png = ?9,
                        image_original_bytes = ?10,
                        image_original_mime = ?11,
                        image_preview_png = ?12,
                        image_width = ?13,
                        image_height = ?14,
                        source_app = ?15,
                        source_icon_data_url = ?16,
                        hash = ?17,
                        pinned = ?18,
                        favorite = ?19
                    WHERE id = ?1
                    "#,
                    params![
                        next.id,
                        next.kind,
                        next.created_at,
                        next.pinned_at,
                        next.preview,
                        next.full_text,
                        next.html_text,
                        next.rtf_text,
                        next.image_png,
                        next.image_original_bytes,
                        next.image_original_mime,
                        next.image_preview_png,
                        next.image_width,
                        next.image_height,
                        next.source_app,
                        next.source_icon_data_url,
                        next.hash,
                        next.pinned as i64,
                        next.favorite as i64
                    ],
                )?;
                trim_history(&tx, settings.max_history_items)?;
                let item = next;
                tx.commit()?;
                Ok(UpsertedCapture {
                    inserted: false,
                    item,
                })
            }
            None => {
                let item = build_new_item(capture, &now, source_app_name, source_icon_data_url);
                tx.execute(
                    r#"
                    INSERT INTO clipboard_items (
                      id, kind, created_at, pinned_at, preview, full_text, html_text, rtf_text,
                      image_png, image_original_bytes, image_original_mime, image_preview_png,
                      image_width, image_height, source_app, source_icon_data_url,
                      hash, pinned, favorite
                    ) VALUES (
                      ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
                    )
                    "#,
                    params![
                        item.id,
                        item.kind,
                        item.created_at,
                        item.pinned_at,
                        item.preview,
                        item.full_text,
                        item.html_text,
                        item.rtf_text,
                        item.image_png,
                        item.image_original_bytes,
                        item.image_original_mime,
                        item.image_preview_png,
                        item.image_width,
                        item.image_height,
                        item.source_app,
                        item.source_icon_data_url,
                        item.hash,
                        item.pinned as i64,
                        item.favorite as i64
                    ],
                )?;
                trim_history(&tx, settings.max_history_items)?;
                let item = item;
                tx.commit()?;
                Ok(UpsertedCapture {
                    inserted: true,
                    item,
                })
            }
        }
    }

    pub(crate) fn toggle_pin(&self, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.connection.execute(
            r#"
            UPDATE clipboard_items
            SET pinned = CASE pinned WHEN 0 THEN 1 ELSE 0 END,
                pinned_at = CASE pinned WHEN 0 THEN ?2 ELSE NULL END
            WHERE id = ?1
            "#,
            params![id, now],
        )?;
        Ok(())
    }

    pub(crate) fn toggle_favorite(&self, id: &str) -> Result<()> {
        self.connection.execute(
            r#"
            UPDATE clipboard_items
            SET favorite = CASE favorite WHEN 0 THEN 1 ELSE 0 END
            WHERE id = ?1
            "#,
            params![id],
        )?;
        Ok(())
    }

    pub(crate) fn delete_item(&self, id: &str) -> Result<()> {
        self.connection
            .execute("DELETE FROM clipboard_items WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub(crate) fn update_text_item(&self, id: &str, text: &str) -> Result<()> {
        let item = self
            .get_item(id)?
            .ok_or_else(|| anyhow::anyhow!("Clipboard item not found"))?;
        if item.kind != "text" {
            anyhow::bail!("Only text items can be edited");
        }

        self.connection.execute(
            r#"
            UPDATE clipboard_items
            SET full_text = ?2,
                html_text = NULL,
                rtf_text = NULL,
                preview = ?3,
                hash = ?4,
                created_at = ?5
            WHERE id = ?1
            "#,
            params![
                id,
                text,
                preview_text(text),
                sha256_hex(text.as_bytes()),
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub(crate) fn update_source_icon(&self, id: &str, source_icon_data_url: &str) -> Result<()> {
        self.connection.execute(
            r#"
            UPDATE clipboard_items
            SET source_icon_data_url = ?2
            WHERE id = ?1
            "#,
            params![id, source_icon_data_url],
        )?;
        Ok(())
    }

    pub(crate) fn clear_history(&self) -> Result<()> {
        self.connection
            .execute("DELETE FROM clipboard_items WHERE pinned = 0", [])?;
        Ok(())
    }

    fn row_to_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredClipboardItem> {
        let mut item = StoredClipboardItem {
            id: row.get(0)?,
            kind: row.get(1)?,
            created_at: row.get(2)?,
            pinned_at: row.get(3)?,
            preview: row.get(4)?,
            full_text: row.get(5)?,
            html_text: row.get(6)?,
            rtf_text: row.get(7)?,
            image_png: row.get(8)?,
            image_original_bytes: row.get(9)?,
            image_original_mime: row.get(10)?,
            image_preview_png: row.get(11)?,
            image_width: row.get(12)?,
            image_height: row.get(13)?,
            source_app: row.get(14)?,
            source_icon_data_url: row.get(15)?,
            hash: row.get(16)?,
            pinned: row.get::<_, i64>(17)? != 0,
            favorite: row.get::<_, i64>(18)? != 0,
        };

        let (full_text, html_text) =
            normalize_rich_text_payload(item.full_text.take(), item.html_text.take());
        item.full_text = full_text;
        item.html_text = html_text;

        if matches!(item.kind.as_str(), "text" | "link" | "mixed") {
            if let Some(text) = item.full_text.as_deref() {
                item.preview = preview_text(text);
            }
        }

        Ok(item)
    }
}

fn capture_hash(capture: &CapturedClipboard) -> &str {
    match capture {
        CapturedClipboard::Text { hash, .. }
        | CapturedClipboard::Link { hash, .. }
        | CapturedClipboard::Image { hash, .. }
        | CapturedClipboard::Mixed { hash, .. } => hash,
    }
}

fn capture_matching_text(capture: &CapturedClipboard) -> Option<String> {
    match capture {
        CapturedClipboard::Text { text, .. }
        | CapturedClipboard::Link { text, .. }
        | CapturedClipboard::Mixed { text, .. }
            if !text.is_empty() =>
        {
            Some(text.clone())
        }
        _ => None,
    }
}

fn apply_capture(
    mut item: StoredClipboardItem,
    capture: CapturedClipboard,
    now: &str,
    source_app: Option<String>,
    source_icon_data_url: Option<String>,
) -> StoredClipboardItem {
    item.created_at = now.to_string();
    item.source_app = source_app;
    item.source_icon_data_url = source_icon_data_url;

    match capture {
        CapturedClipboard::Text {
            text,
            html_text,
            rtf_text,
            hash,
        } => {
            item.kind = "text".into();
            item.preview = preview_text(&text);
            item.full_text = Some(text);
            item.html_text = html_text;
            item.rtf_text = rtf_text;
            item.image_png = None;
            item.image_original_bytes = None;
            item.image_original_mime = None;
            item.image_preview_png = None;
            item.image_width = None;
            item.image_height = None;
            item.hash = hash;
        }
        CapturedClipboard::Link {
            text,
            html_text,
            rtf_text,
            hash,
        } => {
            item.kind = "link".into();
            item.preview = preview_text(&text);
            item.full_text = Some(text);
            item.html_text = html_text;
            item.rtf_text = rtf_text;
            item.image_png = None;
            item.image_original_bytes = None;
            item.image_original_mime = None;
            item.image_preview_png = None;
            item.image_width = None;
            item.image_height = None;
            item.hash = hash;
        }
        CapturedClipboard::Image {
            png_bytes,
            original_bytes,
            original_mime,
            hash,
            preview,
            image_width,
            image_height,
        } => {
            item.kind = "image".into();
            item.preview = preview;
            item.full_text = None;
            item.html_text = None;
            item.rtf_text = None;
            item.image_preview_png = image_preview_png_from_bytes(
                original_bytes.as_deref().unwrap_or(png_bytes.as_slice()),
            );
            item.image_png = Some(png_bytes);
            item.image_original_bytes = original_bytes;
            item.image_original_mime = original_mime;
            item.image_width = Some(image_width);
            item.image_height = Some(image_height);
            item.hash = hash;
        }
        CapturedClipboard::Mixed {
            text,
            html_text,
            rtf_text,
            png_bytes,
            hash,
            image_width,
            image_height,
        } => {
            item.kind = "mixed".into();
            item.preview = preview_text(&text);
            item.full_text = Some(text);
            item.html_text = html_text;
            item.rtf_text = rtf_text;
            item.image_preview_png = png_bytes.as_deref().and_then(image_preview_png_from_bytes);
            item.image_png = png_bytes;
            item.image_original_bytes = None;
            item.image_original_mime = None;
            item.image_width = (image_width > 0).then_some(image_width);
            item.image_height = (image_height > 0).then_some(image_height);
            item.hash = hash;
        }
    }

    item
}

fn build_new_item(
    capture: CapturedClipboard,
    now: &str,
    source_app: Option<String>,
    source_icon_data_url: Option<String>,
) -> StoredClipboardItem {
    let item = StoredClipboardItem {
        id: Uuid::new_v4().to_string(),
        kind: String::new(),
        created_at: now.to_string(),
        pinned_at: None,
        preview: String::new(),
        full_text: None,
        html_text: None,
        rtf_text: None,
        image_png: None,
        image_original_bytes: None,
        image_original_mime: None,
        image_preview_png: None,
        image_width: None,
        image_height: None,
        source_app,
        source_icon_data_url,
        hash: String::new(),
        pinned: false,
        favorite: false,
    };

    let source_app = item.source_app.clone();
    let source_icon_data_url = item.source_icon_data_url.clone();
    apply_capture(item, capture, now, source_app, source_icon_data_url)
}

fn trim_history(tx: &rusqlite::Transaction<'_>, max_history_items: usize) -> Result<()> {
    let total: i64 = tx.query_row("SELECT COUNT(*) FROM clipboard_items", [], |row| row.get(0))?;
    let overflow = total.saturating_sub(max_history_items as i64);
    if overflow > 0 {
        tx.execute(
            r#"
            DELETE FROM clipboard_items
            WHERE id IN (
              SELECT id
              FROM clipboard_items
              WHERE pinned = 0
              ORDER BY favorite ASC, created_at ASC
              LIMIT ?1
            )
            "#,
            params![overflow],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::SqliteHistoryStore;
    use crate::{
        models::{AppSettings, CapturedClipboard, StoragePaths},
        storage::sha256_hex,
    };
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    fn test_paths() -> StoragePaths {
        let root = std::env::temp_dir().join(format!("clipdesk-test-{}", Uuid::new_v4()));
        StoragePaths::new(root).expect("storage paths")
    }

    fn text_capture(text: &str) -> CapturedClipboard {
        CapturedClipboard::Text {
            text: text.to_string(),
            html_text: None,
            rtf_text: None,
            hash: sha256_hex(text.as_bytes()),
        }
    }

    #[test]
    fn creates_database_and_inserts_rows() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let settings = AppSettings::default();

        let inserted = store
            .upsert_capture(text_capture("alpha"), None, &settings)
            .expect("insert");
        assert!(inserted.inserted);
        assert!(PathBuf::from(&paths.db_path).exists());
        assert_eq!(store.list_all().expect("all").len(), 1);

        let _ = fs::remove_dir_all(paths.db_path.parent().unwrap_or(paths.db_path.as_path()));
    }

    #[test]
    fn deduplicates_same_text_content() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let settings = AppSettings::default();

        assert!(
            store
                .upsert_capture(text_capture("alpha"), None, &settings)
                .expect("first")
                .inserted
        );
        assert!(
            !store
                .upsert_capture(text_capture("alpha"), None, &settings)
                .expect("second")
                .inserted
        );
        assert_eq!(store.list_all().expect("all").len(), 1);

        let _ = fs::remove_dir_all(paths.db_path.parent().unwrap_or(paths.db_path.as_path()));
    }

    #[test]
    fn evicts_oldest_unpinned_items() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let mut settings = AppSettings::default();
        settings.max_history_items = 2;

        store
            .upsert_capture(text_capture("alpha"), None, &settings)
            .expect("alpha");
        store
            .upsert_capture(text_capture("beta"), None, &settings)
            .expect("beta");
        store
            .upsert_capture(text_capture("gamma"), None, &settings)
            .expect("gamma");

        let items = store.list_all().expect("all");
        assert_eq!(items.len(), 2);
        assert!(items
            .iter()
            .all(|item| item.full_text.as_deref() != Some("alpha")));

        let _ = fs::remove_dir_all(paths.db_path.parent().unwrap_or(paths.db_path.as_path()));
    }

    #[test]
    fn deletes_and_clears_unpinned_rows() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let settings = AppSettings::default();

        store
            .upsert_capture(text_capture("alpha"), None, &settings)
            .expect("alpha");
        store
            .upsert_capture(text_capture("beta"), None, &settings)
            .expect("beta");

        let id = store
            .list_all()
            .expect("all")
            .into_iter()
            .find(|item| item.full_text.as_deref() == Some("alpha"))
            .expect("alpha item")
            .id;
        store.toggle_pin(&id).expect("pin");
        store.clear_history().expect("clear");

        let items = store.list_all().expect("all");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, id);

        store.delete_item(&id).expect("delete");
        assert!(store.list_all().expect("empty").is_empty());

        let _ = fs::remove_dir_all(paths.db_path.parent().unwrap_or(paths.db_path.as_path()));
    }

    #[test]
    fn updates_text_items() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let settings = AppSettings::default();

        store
            .upsert_capture(text_capture("alpha"), None, &settings)
            .expect("alpha");
        let id = store.list_all().expect("all")[0].id.clone();

        store.update_text_item(&id, "beta").expect("update");
        let item = store.get_item(&id).expect("item").expect("row");
        assert_eq!(item.full_text.as_deref(), Some("beta"));
        assert_eq!(item.hash, sha256_hex("beta".as_bytes()));

        let _ = fs::remove_dir_all(paths.db_path.parent().unwrap_or(paths.db_path.as_path()));
    }

    #[test]
    fn updates_source_app_when_same_content_is_captured_again() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let settings = AppSettings::default();

        store
            .upsert_capture(
                text_capture("alpha"),
                Some(("PixPin".into(), Some("pixpin-icon".into()))),
                &settings,
            )
            .expect("first");
        store
            .upsert_capture(
                text_capture("alpha"),
                Some(("DingTalk".into(), Some("dingtalk-icon".into()))),
                &settings,
            )
            .expect("second");

        let item = store.list_all().expect("all").remove(0);
        assert_eq!(item.source_app.as_deref(), Some("DingTalk"));
        assert_eq!(item.source_icon_data_url.as_deref(), Some("dingtalk-icon"));

        let _ = fs::remove_dir_all(paths.db_path.parent().unwrap_or(paths.db_path.as_path()));
    }

    #[test]
    fn stores_mixed_items_without_binary_image_payload() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let settings = AppSettings::default();

        store
            .upsert_capture(
                CapturedClipboard::Mixed {
                    text: "hello".into(),
                    html_text: Some("<p>hello</p><img src=\"cid:test\" />".into()),
                    rtf_text: None,
                    png_bytes: None,
                    hash: sha256_hex(b"hello"),
                    image_width: 0,
                    image_height: 0,
                },
                None,
                &settings,
            )
            .expect("insert");

        let item = store.list_all().expect("all").remove(0);
        assert_eq!(item.kind, "mixed");
        assert_eq!(item.full_text.as_deref(), Some("hello"));
        assert!(item.image_png.is_none());
        assert!(item
            .html_text
            .as_deref()
            .unwrap_or_default()
            .contains("<img"));

        let _ = fs::remove_dir_all(paths.db_path.parent().unwrap_or(paths.db_path.as_path()));
    }

    #[test]
    fn upgrades_existing_text_item_to_mixed_when_html_contains_image() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let settings = AppSettings::default();

        store
            .upsert_capture(text_capture("hello"), None, &settings)
            .expect("text");
        store
            .upsert_capture(
                CapturedClipboard::Mixed {
                    text: "hello".into(),
                    html_text: Some("<p>hello</p><img src=\"cid:test\" />".into()),
                    rtf_text: None,
                    png_bytes: None,
                    hash: sha256_hex(b"hello"),
                    image_width: 0,
                    image_height: 0,
                },
                None,
                &settings,
            )
            .expect("mixed");

        let item = store.list_all().expect("all").remove(0);
        assert_eq!(item.kind, "mixed");
        assert!(item
            .html_text
            .as_deref()
            .unwrap_or_default()
            .contains("<img"));

        let _ = fs::remove_dir_all(paths.db_path.parent().unwrap_or(paths.db_path.as_path()));
    }
}
