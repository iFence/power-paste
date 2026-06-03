use anyhow::{Context, Result};
use base64::Engine;
use chrono::{Duration, Utc};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Transaction};
use serde_json::{from_str, to_string};
use uuid::Uuid;

use crate::{
    models::{
        AppSettings, CapturedClipboard, ClipboardItemDto, DeletedClipboardItem,
        HistoryQueryPayload, StoragePaths, StoredClipboardItem,
    },
    history::html_image_preview_data_url,
    rich_text::normalize_rich_text_payload,
    sensitive_text::{build_sensitive_text_mask, SensitiveTextMask},
    storage::{image_preview_png_from_bytes, preview_text, sha256_hex},
};

pub(crate) struct SqliteHistoryStore {
    connection: Connection,
}

pub(crate) struct UpsertedCapture {
    pub(crate) inserted: bool,
    pub(crate) item: StoredClipboardItem,
}

struct HistoryListRow {
    id: String,
    kind: String,
    created_at: String,
    preview: String,
    full_text: Option<String>,
    html_text: Option<String>,
    image_preview_png: Option<Vec<u8>>,
    image_width: Option<u32>,
    image_height: Option<u32>,
    image_byte_size: Option<usize>,
    source_app: Option<String>,
    source_icon_data_url: Option<String>,
    pinned: bool,
    favorite: bool,
    tag_colors: Vec<String>,
    copy_count: u64,
}

const ALLOWED_TAG_COLORS: [&str; 7] = ["red", "orange", "yellow", "green", "blue", "purple", "gray"];
const DEVICE_ID_KEY: &str = "device_id";
const LAST_SYNC_AT_KEY: &str = "last_sync_at";

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
              favorite INTEGER NOT NULL DEFAULT 0,
              tag_colors TEXT NOT NULL DEFAULT '[]',
              copy_count INTEGER NOT NULL DEFAULT 0,
              updated_at TEXT NOT NULL DEFAULT '',
              sync_updated_at TEXT NOT NULL DEFAULT '',
              sync_device_id TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS deleted_clipboard_items (
              id TEXT PRIMARY KEY NOT NULL,
              deleted_at TEXT NOT NULL,
              sync_updated_at TEXT NOT NULL,
              sync_device_id TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sync_metadata (
              key TEXT PRIMARY KEY NOT NULL,
              value TEXT NOT NULL
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
        ensure_column(&self.connection, "tag_colors", "TEXT NOT NULL DEFAULT '[]'")?;
        ensure_column(&self.connection, "copy_count", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(&self.connection, "updated_at", "TEXT NOT NULL DEFAULT ''")?;
        ensure_column(&self.connection, "sync_updated_at", "TEXT NOT NULL DEFAULT ''")?;
        ensure_column(&self.connection, "sync_device_id", "TEXT NOT NULL DEFAULT ''")?;
        let now = Utc::now().to_rfc3339();
        self.connection.execute(
            "UPDATE clipboard_items SET updated_at = COALESCE(NULLIF(updated_at, ''), created_at, ?1)",
            params![now],
        )?;
        self.connection.execute(
            "UPDATE clipboard_items SET sync_updated_at = COALESCE(NULLIF(sync_updated_at, ''), updated_at, created_at, ?1)",
            params![now],
        )?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn list_all(&self) -> Result<Vec<StoredClipboardItem>> {
        self.query_items(&HistoryQueryPayload::default(), None, 0)
    }

    pub(crate) fn list_history(
        &self,
        payload: &HistoryQueryPayload,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ClipboardItemDto>> {
        self.query_history_rows(payload, limit, offset)
            .map(|rows| rows.into_iter().map(history_list_row_to_dto).collect())
    }

    pub(crate) fn count_history(&self, payload: &HistoryQueryPayload) -> Result<usize> {
        let (where_sql, bind_values) = build_history_filters(payload);
        let sql = format!("SELECT COUNT(*) FROM clipboard_items{where_sql}");
        let mut statement = self.connection.prepare(&sql)?;
        statement
            .query_row(params_from_iter(bind_values.iter()), |row| row.get::<_, i64>(0))
            .map(|count| count.max(0) as usize)
            .map_err(Into::into)
    }

    fn query_items(
        &self,
        payload: &HistoryQueryPayload,
        limit: Option<usize>,
        offset: usize,
    ) -> Result<Vec<StoredClipboardItem>> {
        let limit = limit.unwrap_or(i64::MAX as usize).min(i64::MAX as usize) as i64;
        let offset = offset.min(i64::MAX as usize) as i64;
        let (where_sql, mut bind_values) = build_history_filters(payload);
        let sql = format!(
            r#"
            SELECT id, kind, created_at, pinned_at, preview, full_text, html_text, rtf_text,
                   image_png, image_original_bytes, image_original_mime,
                   image_preview_png, image_width, image_height, source_app, source_icon_data_url,
                   hash, pinned, favorite, tag_colors, copy_count, updated_at, sync_updated_at, sync_device_id
            FROM clipboard_items
            {where_sql}
            ORDER BY pinned DESC, pinned_at DESC, favorite DESC, created_at DESC
            LIMIT ?{} OFFSET ?{}
            "#,
            bind_values.len() + 1,
            bind_values.len() + 2,
        );
        bind_values.push(rusqlite::types::Value::Integer(limit));
        bind_values.push(rusqlite::types::Value::Integer(offset));

        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(bind_values.iter()), Self::row_to_item)?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn get_item(&self, id: &str) -> Result<Option<StoredClipboardItem>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT id, kind, created_at, pinned_at, preview, full_text, html_text, rtf_text,
                   image_png, image_original_bytes, image_original_mime,
                   image_preview_png, image_width, image_height, source_app, source_icon_data_url,
                   hash, pinned, favorite, tag_colors, copy_count, updated_at, sync_updated_at, sync_device_id
            FROM clipboard_items
            WHERE id = ?1
            "#,
        )?;
        statement
            .query_row(params![id], Self::row_to_item)
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn local_device_id(&self) -> Result<String> {
        local_device_id(&self.connection)
    }

    pub(crate) fn last_sync_at(&self) -> Result<Option<String>> {
        metadata_value(&self.connection, LAST_SYNC_AT_KEY)
    }

    pub(crate) fn set_last_sync_at(&self, value: &str) -> Result<()> {
        set_metadata_value(&self.connection, LAST_SYNC_AT_KEY, value)
    }

    pub(crate) fn upsert_capture(
        &mut self,
        capture: CapturedClipboard,
        source_app: Option<(String, Option<String>)>,
        settings: &AppSettings,
    ) -> Result<UpsertedCapture> {
        let device_id = self.local_device_id()?;
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
                       hash, pinned, favorite, tag_colors, copy_count, updated_at, sync_updated_at, sync_device_id
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
                       hash, pinned, favorite, tag_colors, copy_count, updated_at, sync_updated_at, sync_device_id
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
                let mut next = apply_capture(
                    existing,
                    capture,
                    &now,
                    source_app_name,
                    source_icon_data_url,
                );
                if settings.copy_stats_enabled {
                    next.copy_count = next.copy_count.saturating_add(1);
                }
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
                        favorite = ?19,
                        tag_colors = ?20,
                        copy_count = ?21,
                        updated_at = ?22,
                        sync_updated_at = ?23,
                        sync_device_id = ?24
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
                        next.favorite as i64,
                        serialize_tag_colors(&next.tag_colors),
                        next.copy_count as i64,
                        now,
                        now,
                        device_id,
                    ],
                )?;
                trim_history(&tx, settings)?;
                let item = next;
                tx.commit()?;
                Ok(UpsertedCapture {
                    inserted: false,
                    item,
                })
            }
            None => {
                let mut item = build_new_item(capture, &now, source_app_name, source_icon_data_url);
                if settings.copy_stats_enabled {
                    item.copy_count = 1;
                }
                tx.execute(
                    r#"
                    INSERT INTO clipboard_items (
                      id, kind, created_at, pinned_at, preview, full_text, html_text, rtf_text,
                      image_png, image_original_bytes, image_original_mime, image_preview_png,
                      image_width, image_height, source_app, source_icon_data_url,
                      hash, pinned, favorite, tag_colors, copy_count, updated_at, sync_updated_at, sync_device_id
                    ) VALUES (
                      ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
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
                        item.favorite as i64,
                        serialize_tag_colors(&item.tag_colors),
                        item.copy_count as i64,
                        now,
                        now,
                        device_id,
                    ],
                )?;
                trim_history(&tx, settings)?;
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
        let device_id = self.local_device_id()?;
        let now = Utc::now().to_rfc3339();
        let affected = self.connection.execute(
            r#"
            UPDATE clipboard_items
            SET pinned = CASE pinned WHEN 0 THEN 1 ELSE 0 END,
                pinned_at = CASE pinned WHEN 0 THEN ?2 ELSE NULL END,
                updated_at = ?2,
                sync_updated_at = ?2,
                sync_device_id = ?3
            WHERE id = ?1
            "#,
            params![id, now, device_id],
        )?;
        if affected == 0 {
            anyhow::bail!("Clipboard item not found");
        }
        Ok(())
    }

    pub(crate) fn toggle_favorite(&self, id: &str) -> Result<()> {
        let device_id = self.local_device_id()?;
        let now = Utc::now().to_rfc3339();
        let affected = self.connection.execute(
            r#"
            UPDATE clipboard_items
            SET favorite = CASE favorite WHEN 0 THEN 1 ELSE 0 END,
                updated_at = ?2,
                sync_updated_at = ?2,
                sync_device_id = ?3
            WHERE id = ?1
            "#,
            params![id, now, device_id],
        )?;
        if affected == 0 {
            anyhow::bail!("Clipboard item not found");
        }
        Ok(())
    }

    pub(crate) fn delete_item(&self, id: &str) -> Result<()> {
        let device_id = self.local_device_id()?;
        let now = Utc::now().to_rfc3339();
        let tx = self.connection.unchecked_transaction()?;
        let affected = tx.execute("DELETE FROM clipboard_items WHERE id = ?1", params![id])?;
        if affected > 0 {
            upsert_deleted_item(&tx, id, &now, &device_id)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn update_text_item(&self, id: &str, text: &str) -> Result<()> {
        let item = self
            .get_item(id)?
            .ok_or_else(|| anyhow::anyhow!("Clipboard item not found"))?;
        if item.kind != "text" {
            anyhow::bail!("Only text items can be edited");
        }

        let device_id = self.local_device_id()?;
        let now = Utc::now().to_rfc3339();
        self.connection.execute(
            r#"
            UPDATE clipboard_items
            SET full_text = ?2,
                html_text = NULL,
                rtf_text = NULL,
                preview = ?3,
                hash = ?4,
                created_at = ?5,
                updated_at = ?5,
                sync_updated_at = ?5,
                sync_device_id = ?6
            WHERE id = ?1
            "#,
            params![
                id,
                text,
                preview_text(text),
                sha256_hex(text.as_bytes()),
                now,
                device_id,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn update_item_tags(&self, id: &str, tag_colors: &[String]) -> Result<()> {
        let device_id = self.local_device_id()?;
        let now = Utc::now().to_rfc3339();
        let next_colors = sanitize_tag_colors(tag_colors);
        let affected = self.connection.execute(
            r#"
            UPDATE clipboard_items
            SET tag_colors = ?2,
                updated_at = ?3,
                sync_updated_at = ?3,
                sync_device_id = ?4
            WHERE id = ?1
            "#,
            params![id, serialize_tag_colors(&next_colors), now, device_id],
        )?;
        if affected == 0 {
            anyhow::bail!("Clipboard item not found");
        }
        Ok(())
    }

    pub(crate) fn update_source_icon(&self, id: &str, source_icon_data_url: &str) -> Result<()> {
        let device_id = self.local_device_id()?;
        let now = Utc::now().to_rfc3339();
        self.connection.execute(
            r#"
            UPDATE clipboard_items
            SET source_icon_data_url = ?2,
                updated_at = ?3,
                sync_updated_at = ?3,
                sync_device_id = ?4
            WHERE id = ?1
            "#,
            params![id, source_icon_data_url, now, device_id],
        )?;
        Ok(())
    }

    pub(crate) fn clear_history(&self) -> Result<()> {
        let device_id = self.local_device_id()?;
        let now = Utc::now().to_rfc3339();
        let tx = self.connection.unchecked_transaction()?;
        let ids = {
            let mut statement = tx.prepare("SELECT id FROM clipboard_items WHERE pinned = 0")?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        tx.execute("DELETE FROM clipboard_items WHERE pinned = 0", [])?;
        for id in ids {
            upsert_deleted_item(&tx, &id, &now, &device_id)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn trim_by_settings(&mut self, settings: &AppSettings) -> Result<usize> {
        let tx = self.connection.transaction()?;
        let removed = trim_history(&tx, settings)?;
        tx.commit()?;
        Ok(removed)
    }

    pub(crate) fn list_sync_items(&self) -> Result<Vec<StoredClipboardItem>> {
        self.query_items(&HistoryQueryPayload::default(), None, 0)
    }

    pub(crate) fn list_deleted_sync_items(&self) -> Result<Vec<DeletedClipboardItem>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT id, deleted_at, sync_updated_at, sync_device_id
            FROM deleted_clipboard_items
            "#,
        )?;
        let rows = statement.query_map([], |row| {
            Ok(DeletedClipboardItem {
                id: row.get(0)?,
                deleted_at: row.get(1)?,
                sync_updated_at: row.get(2)?,
                sync_device_id: row.get(3)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn upsert_synced_item(&self, item: &StoredClipboardItem) -> Result<bool> {
        let tx = self.connection.unchecked_transaction()?;
        let existing = {
            let mut statement = tx.prepare(
                "SELECT sync_updated_at, sync_device_id FROM clipboard_items WHERE id = ?1",
            )?;
            statement
                .query_row(params![item.id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .optional()?
        };
        if let Some((updated_at, device_id)) = existing {
            if !is_remote_newer(
                &item.sync_updated_at,
                &item.sync_device_id,
                &updated_at,
                &device_id,
            ) {
                return Ok(false);
            }
        }

        tx.execute(
            "DELETE FROM deleted_clipboard_items WHERE id = ?1",
            params![item.id],
        )?;
        tx.execute(
            r#"
            INSERT INTO clipboard_items (
              id, kind, created_at, pinned_at, preview, full_text, html_text, rtf_text,
              image_png, image_original_bytes, image_original_mime, image_preview_png,
              image_width, image_height, source_app, source_icon_data_url,
              hash, pinned, favorite, tag_colors, updated_at, sync_updated_at, sync_device_id
            ) VALUES (
              ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
            )
            ON CONFLICT(id) DO UPDATE SET
              kind = excluded.kind,
              created_at = excluded.created_at,
              pinned_at = excluded.pinned_at,
              preview = excluded.preview,
              full_text = excluded.full_text,
              html_text = excluded.html_text,
              rtf_text = excluded.rtf_text,
              image_png = excluded.image_png,
              image_original_bytes = excluded.image_original_bytes,
              image_original_mime = excluded.image_original_mime,
              image_preview_png = excluded.image_preview_png,
              image_width = excluded.image_width,
              image_height = excluded.image_height,
              source_app = excluded.source_app,
              source_icon_data_url = excluded.source_icon_data_url,
              hash = excluded.hash,
              pinned = excluded.pinned,
              favorite = excluded.favorite,
              tag_colors = excluded.tag_colors,
              updated_at = excluded.updated_at,
              sync_updated_at = excluded.sync_updated_at,
              sync_device_id = excluded.sync_device_id
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
                item.favorite as i64,
                serialize_tag_colors(&item.tag_colors),
                item.updated_at,
                item.sync_updated_at,
                item.sync_device_id,
            ],
        )?;
        tx.commit()?;
        Ok(true)
    }

    pub(crate) fn increment_copy_count(&self, id: &str) -> Result<ClipboardItemDto> {
        let affected = self.connection.execute(
            "UPDATE clipboard_items SET copy_count = copy_count + 1 WHERE id = ?1",
            params![id],
        )?;
        if affected == 0 {
            anyhow::bail!("Clipboard item not found");
        }

        self.get_history_item(id)?
            .ok_or_else(|| anyhow::anyhow!("Clipboard item not found"))
    }

    pub(crate) fn apply_synced_deletion(&self, deletion: &DeletedClipboardItem) -> Result<bool> {
        let existing = self.get_item(&deletion.id)?;
        let should_delete = existing.as_ref().is_some_and(|item| {
            is_remote_newer(
                &deletion.sync_updated_at,
                &deletion.sync_device_id,
                &item.sync_updated_at,
                &item.sync_device_id,
            )
        });
        let existing_tombstone = self
            .list_deleted_sync_items()?
            .into_iter()
            .find(|item| item.id == deletion.id);
        let should_store_tombstone = existing_tombstone.as_ref().map_or(true, |item| {
            is_remote_newer(
                &deletion.sync_updated_at,
                &deletion.sync_device_id,
                &item.sync_updated_at,
                &item.sync_device_id,
            )
        });

        if !should_delete && !should_store_tombstone {
            return Ok(false);
        }

        let tx = self.connection.unchecked_transaction()?;
        if should_delete {
            tx.execute("DELETE FROM clipboard_items WHERE id = ?1", params![deletion.id])?;
        }
        if should_store_tombstone {
            tx.execute(
                r#"
                INSERT INTO deleted_clipboard_items (id, deleted_at, sync_updated_at, sync_device_id)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(id) DO UPDATE SET
                  deleted_at = excluded.deleted_at,
                  sync_updated_at = excluded.sync_updated_at,
                  sync_device_id = excluded.sync_device_id
                "#,
                params![
                    deletion.id,
                    deletion.deleted_at,
                    deletion.sync_updated_at,
                    deletion.sync_device_id,
                ],
            )?;
        }
        tx.commit()?;
        Ok(should_delete || should_store_tombstone)
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
            tag_colors: parse_tag_colors(row.get::<_, String>(19)?),
            copy_count: row
                .get::<_, i64>(20)
                .map(|value| u64::try_from(value.max(0)).unwrap_or(0))?,
            updated_at: row.get(21)?,
            sync_updated_at: row.get(22)?,
            sync_device_id: row.get(23)?,
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

    fn query_history_rows(
        &self,
        payload: &HistoryQueryPayload,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<HistoryListRow>> {
        let limit = limit.min(i64::MAX as usize) as i64;
        let offset = offset.min(i64::MAX as usize) as i64;
        let (where_sql, mut bind_values) = build_history_filters(payload);
        let order_sql = if payload.copy_stats_enabled {
            "ORDER BY pinned DESC, pinned_at DESC, copy_count DESC, created_at DESC"
        } else {
            "ORDER BY pinned DESC, pinned_at DESC, favorite DESC, created_at DESC"
        };
        let sql = format!(
            r#"
            SELECT id, kind, created_at, preview, full_text, html_text,
                   CASE
                     WHEN image_preview_png IS NOT NULL AND length(image_preview_png) > 0
                       THEN image_preview_png
                     ELSE image_png
                   END AS image_list_png,
                   image_width, image_height,
                   length(COALESCE(image_original_bytes, image_png)),
                   source_app, source_icon_data_url,
                   pinned, favorite, tag_colors, copy_count
            FROM clipboard_items
            {where_sql}
            {order_sql}
            LIMIT ?{} OFFSET ?{}
            "#,
            bind_values.len() + 1,
            bind_values.len() + 2,
        );
        bind_values.push(rusqlite::types::Value::Integer(limit));
        bind_values.push(rusqlite::types::Value::Integer(offset));

        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(
            params_from_iter(bind_values.iter()),
            Self::row_to_history_list_row,
        )?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn row_to_history_list_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryListRow> {
        let mut full_text = row.get::<_, Option<String>>(4)?;
        let mut html_text = row.get::<_, Option<String>>(5)?;
        let (normalized_full_text, normalized_html_text) =
            normalize_rich_text_payload(full_text.take(), html_text.take());

        let kind = row.get::<_, String>(1)?;
        let stored_preview = row.get::<_, String>(3)?;
        let preview = if matches!(kind.as_str(), "text" | "link" | "mixed") {
            normalized_full_text
                .as_deref()
                .map(preview_text)
                .unwrap_or(stored_preview)
        } else {
            stored_preview
        };

        Ok(HistoryListRow {
            id: row.get(0)?,
            kind,
            created_at: row.get(2)?,
            preview,
            full_text: normalized_full_text,
            html_text: normalized_html_text,
            image_preview_png: row.get(6)?,
            image_width: row.get(7)?,
            image_height: row.get(8)?,
            image_byte_size: row
                .get::<_, Option<i64>>(9)?
                .and_then(|value| usize::try_from(value).ok()),
            source_app: row.get(10)?,
            source_icon_data_url: row.get(11)?,
            pinned: row.get::<_, i64>(12)? != 0,
            favorite: row.get::<_, i64>(13)? != 0,
            tag_colors: parse_tag_colors(row.get::<_, String>(14)?),
            copy_count: row
                .get::<_, i64>(15)
                .map(|value| u64::try_from(value.max(0)).unwrap_or(0))?,
        })
    }

    fn get_history_item(&self, id: &str) -> Result<Option<ClipboardItemDto>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT id, kind, created_at, preview, full_text, html_text,
                   CASE
                     WHEN image_preview_png IS NOT NULL AND length(image_preview_png) > 0
                       THEN image_preview_png
                     ELSE image_png
                   END AS image_list_png,
                   image_width, image_height,
                   length(COALESCE(image_original_bytes, image_png)),
                   source_app, source_icon_data_url,
                   pinned, favorite, tag_colors, copy_count
            FROM clipboard_items
            WHERE id = ?1
            "#,
        )?;
        statement
            .query_row(params![id], Self::row_to_history_list_row)
            .optional()
            .map(|item| item.map(history_list_row_to_dto))
            .map_err(Into::into)
    }
}

fn history_list_row_to_dto(item: HistoryListRow) -> ClipboardItemDto {
    let image_data_url = image_data_url_from_bytes(item.image_preview_png.as_deref()).or_else(|| {
        item.html_text
            .as_deref()
            .filter(|_| item.kind == "mixed")
            .and_then(html_image_preview_data_url)
    });
    let sensitive_mask =
        build_sensitive_text_mask(&item.kind, &item.preview, item.full_text.as_deref());

    ClipboardItemDto {
        id: item.id,
        kind: item.kind,
        created_at: item.created_at,
        preview: item.preview,
        full_text: item.full_text,
        is_sensitive: sensitive_mask.is_sensitive,
        masked_preview: sensitive_mask.masked_preview,
        masked_full_text: sensitive_mask.masked_full_text,
        image_data_url,
        image_byte_size: item.image_byte_size,
        image_width: item.image_width,
        image_height: item.image_height,
        source_app: item.source_app,
        source_icon_data_url: item.source_icon_data_url,
        pinned: item.pinned,
        favorite: item.favorite,
        tag_colors: item.tag_colors,
        copy_count: item.copy_count,
    }
}

fn image_data_url_from_bytes(bytes: Option<&[u8]>) -> Option<String> {
    bytes
        .filter(|value| !value.is_empty())
        .map(|value| format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(value)))
}

fn build_history_filters(payload: &HistoryQueryPayload) -> (String, Vec<rusqlite::types::Value>) {
    let mut clauses = Vec::new();
    let mut bind_values = Vec::new();

    if let Some(query) = payload
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        clauses.push(
            "lower(preview || char(10) || coalesce(full_text, '') || char(10) || coalesce(source_app, '')) LIKE '%' || ?1 || '%'".to_string(),
        );
        bind_values.push(rusqlite::types::Value::Text(query.to_lowercase()));
    }

    if let Some(kind) = payload
        .kind
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let placeholder = bind_values.len() + 1;
        match kind {
            "text" => {
                clauses.push(format!("kind IN (?{placeholder}, ?{})", placeholder + 1));
                bind_values.push(rusqlite::types::Value::Text("text".into()));
                bind_values.push(rusqlite::types::Value::Text("link".into()));
            }
            "image" | "mixed" | "link" => {
                clauses.push(format!("kind = ?{placeholder}"));
                bind_values.push(rusqlite::types::Value::Text(kind.to_string()));
            }
            _ => {}
        }
    }

    if payload.pinned_only {
        clauses.push("pinned = 1".to_string());
    }

    if let Some(tag_color) = payload
        .tag_color
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let placeholder = bind_values.len() + 1;
        clauses.push(format!("tag_colors LIKE '%' || ?{placeholder} || '%'"));
        bind_values.push(rusqlite::types::Value::Text(format!("\"{}\"", tag_color.to_ascii_lowercase())));
    }

    if clauses.is_empty() {
        (String::new(), bind_values)
    } else {
        (format!(" WHERE {}", clauses.join(" AND ")), bind_values)
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

    apply_sensitive_red_tag(&mut item);

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
        tag_colors: Vec::new(),
        copy_count: 0,
        updated_at: now.to_string(),
        sync_updated_at: now.to_string(),
        sync_device_id: String::new(),
    };

    let source_app = item.source_app.clone();
    let source_icon_data_url = item.source_icon_data_url.clone();
    apply_capture(item, capture, now, source_app, source_icon_data_url)
}

fn sanitize_tag_colors(colors: &[String]) -> Vec<String> {
    let mut next = Vec::new();
    for color in colors {
        let normalized = color.trim().to_ascii_lowercase();
        if !ALLOWED_TAG_COLORS.contains(&normalized.as_str()) {
            continue;
        }
        if next.iter().any(|existing| existing == &normalized) {
            continue;
        }
        next.push(normalized);
        if next.len() == 3 {
            break;
        }
    }
    next
}

fn apply_sensitive_red_tag(item: &mut StoredClipboardItem) {
    let SensitiveTextMask { is_sensitive, .. } =
        build_sensitive_text_mask(&item.kind, &item.preview, item.full_text.as_deref());
    if !is_sensitive {
        return;
    }

    let mut next_colors = vec!["red".to_string()];
    for color in &item.tag_colors {
        if color.trim().eq_ignore_ascii_case("red") {
            continue;
        }
        next_colors.push(color.clone());
        if next_colors.len() == 3 {
            break;
        }
    }
    item.tag_colors = sanitize_tag_colors(&next_colors);
}

fn serialize_tag_colors(colors: &[String]) -> String {
    to_string(&sanitize_tag_colors(colors)).unwrap_or_else(|_| "[]".to_string())
}

fn parse_tag_colors(raw: String) -> Vec<String> {
    from_str::<Vec<String>>(&raw)
        .map(|colors| sanitize_tag_colors(&colors))
        .unwrap_or_default()
}

fn metadata_value(connection: &Connection, key: &str) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT value FROM sync_metadata WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

fn set_metadata_value(connection: &Connection, key: &str, value: &str) -> Result<()> {
    connection.execute(
        r#"
        INSERT INTO sync_metadata (key, value)
        VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
        params![key, value],
    )?;
    Ok(())
}

fn local_device_id(connection: &Connection) -> Result<String> {
    if let Some(value) = metadata_value(connection, DEVICE_ID_KEY)? {
        return Ok(value);
    }

    let device_id = Uuid::new_v4().to_string();
    set_metadata_value(connection, DEVICE_ID_KEY, &device_id)?;
    connection.execute(
        "UPDATE clipboard_items SET sync_device_id = ?1 WHERE sync_device_id = ''",
        params![device_id],
    )?;
    Ok(device_id)
}

fn upsert_deleted_item(
    tx: &Transaction<'_>,
    id: &str,
    deleted_at: &str,
    device_id: &str,
) -> Result<()> {
    tx.execute(
        r#"
        INSERT INTO deleted_clipboard_items (id, deleted_at, sync_updated_at, sync_device_id)
        VALUES (?1, ?2, ?2, ?3)
        ON CONFLICT(id) DO UPDATE SET
          deleted_at = excluded.deleted_at,
          sync_updated_at = excluded.sync_updated_at,
          sync_device_id = excluded.sync_device_id
        "#,
        params![id, deleted_at, device_id],
    )?;
    Ok(())
}

fn is_remote_newer(
    remote_updated_at: &str,
    remote_device_id: &str,
    local_updated_at: &str,
    local_device_id: &str,
) -> bool {
    match remote_updated_at.cmp(local_updated_at) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Equal => remote_device_id > local_device_id,
        std::cmp::Ordering::Less => false,
    }
}

fn trim_history(tx: &rusqlite::Transaction<'_>, settings: &AppSettings) -> Result<usize> {
    let mut removed = 0;
    if settings.max_history_days > 0 {
        let cutoff = Utc::now()
            .checked_sub_signed(Duration::days(settings.max_history_days as i64))
            .unwrap_or_else(Utc::now)
            .to_rfc3339();
        removed += tx.execute(
            r#"
            DELETE FROM clipboard_items
            WHERE pinned = 0
              AND created_at < ?1
            "#,
            params![cutoff],
        )?;
    }

    let total: i64 = tx.query_row("SELECT COUNT(*) FROM clipboard_items", [], |row| row.get(0))?;
    let overflow = total.saturating_sub(settings.max_history_items as i64);
    if overflow > 0 {
        removed += tx.execute(
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
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::SqliteHistoryStore;
    use crate::{
        models::{AppSettings, CapturedClipboard, StoragePaths},
        storage::sha256_hex,
    };
    use chrono::{Duration, Utc};
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
        let items = store.list_all().expect("all");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].copy_count, 0);

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
    fn increments_copy_count_for_existing_items() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let settings = AppSettings::default();

        store
            .upsert_capture(text_capture("alpha"), None, &settings)
            .expect("alpha");
        let id = store.list_all().expect("all")[0].id.clone();

        let first = store.increment_copy_count(&id).expect("first copy");
        let second = store.increment_copy_count(&id).expect("second copy");

        assert_eq!(first.copy_count, 1);
        assert_eq!(second.copy_count, 2);
        assert_eq!(
            store.get_item(&id).expect("item").expect("row").copy_count,
            2
        );

        let _ = fs::remove_dir_all(paths.db_path.parent().unwrap_or(paths.db_path.as_path()));
    }

    #[test]
    fn counts_captured_copies_when_stats_are_enabled() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let mut settings = AppSettings::default();
        settings.copy_stats_enabled = true;

        let first = store
            .upsert_capture(text_capture("retrying"), None, &settings)
            .expect("first");
        let second = store
            .upsert_capture(text_capture("retrying"), None, &settings)
            .expect("second");

        assert!(first.inserted);
        assert!(!second.inserted);
        assert_eq!(first.item.copy_count, 1);
        assert_eq!(second.item.copy_count, 2);
        assert_eq!(store.list_all().expect("all")[0].copy_count, 2);

        let _ = fs::remove_dir_all(paths.db_path.parent().unwrap_or(paths.db_path.as_path()));
    }

    #[test]
    fn sorts_by_copy_count_when_stats_are_enabled() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let settings = AppSettings::default();

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
        let alpha_id = items
            .iter()
            .find(|item| item.full_text.as_deref() == Some("alpha"))
            .expect("alpha item")
            .id
            .clone();
        let beta_id = items
            .iter()
            .find(|item| item.full_text.as_deref() == Some("beta"))
            .expect("beta item")
            .id
            .clone();

        store.increment_copy_count(&beta_id).expect("copy beta");
        store.increment_copy_count(&alpha_id).expect("copy alpha once");
        store.increment_copy_count(&alpha_id).expect("copy alpha twice");

        let payload = crate::models::HistoryQueryPayload {
            copy_stats_enabled: true,
            ..Default::default()
        };
        let history = store.list_history(&payload, 10, 0).expect("history");

        assert_eq!(history[0].full_text.as_deref(), Some("alpha"));
        assert_eq!(history[0].copy_count, 2);
        assert_eq!(history[1].full_text.as_deref(), Some("beta"));
        assert_eq!(history[1].copy_count, 1);

        let _ = fs::remove_dir_all(paths.db_path.parent().unwrap_or(paths.db_path.as_path()));
    }

    #[test]
    fn keeps_pinned_items_before_copy_count_sort() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let settings = AppSettings::default();

        store
            .upsert_capture(text_capture("alpha"), None, &settings)
            .expect("alpha");
        store
            .upsert_capture(text_capture("beta"), None, &settings)
            .expect("beta");
        let items = store.list_all().expect("all");
        let alpha_id = items
            .iter()
            .find(|item| item.full_text.as_deref() == Some("alpha"))
            .expect("alpha item")
            .id
            .clone();
        let beta_id = items
            .iter()
            .find(|item| item.full_text.as_deref() == Some("beta"))
            .expect("beta item")
            .id
            .clone();

        store.toggle_pin(&beta_id).expect("pin beta");
        store.increment_copy_count(&alpha_id).expect("copy alpha once");
        store.increment_copy_count(&alpha_id).expect("copy alpha twice");

        let payload = crate::models::HistoryQueryPayload {
            copy_stats_enabled: true,
            ..Default::default()
        };
        let history = store.list_history(&payload, 10, 0).expect("history");

        assert_eq!(history[0].full_text.as_deref(), Some("beta"));
        assert!(history[0].pinned);
        assert_eq!(history[1].full_text.as_deref(), Some("alpha"));
        assert_eq!(history[1].copy_count, 2);

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
    fn evicts_unpinned_items_older_than_retention_days() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let mut settings = AppSettings::default();
        settings.max_history_days = 30;

        store
            .upsert_capture(text_capture("old"), None, &settings)
            .expect("old");
        store
            .upsert_capture(text_capture("new"), None, &settings)
            .expect("new");
        let old_created_at = (Utc::now() - Duration::days(31)).to_rfc3339();
        store
            .connection
            .execute(
                "UPDATE clipboard_items SET created_at = ?1 WHERE full_text = 'old'",
                [old_created_at],
            )
            .expect("age old item");

        let removed = store.trim_by_settings(&settings).expect("trim");
        let items = store.list_all().expect("all");

        assert_eq!(removed, 1);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].full_text.as_deref(), Some("new"));

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
    fn records_deleted_item_tombstone_for_sync() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let settings = AppSettings::default();

        store
            .upsert_capture(text_capture("alpha"), None, &settings)
            .expect("alpha");
        let id = store.list_all().expect("all")[0].id.clone();

        store.delete_item(&id).expect("delete");

        let deleted = store.list_deleted_sync_items().expect("deleted");
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].id, id);
        assert!(!deleted[0].sync_device_id.is_empty());

        let _ = fs::remove_dir_all(paths.db_path.parent().unwrap_or(paths.db_path.as_path()));
    }

    #[test]
    fn applies_newer_synced_item() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let settings = AppSettings::default();

        store
            .upsert_capture(text_capture("alpha"), None, &settings)
            .expect("alpha");
        let mut item = store.list_all().expect("all").remove(0);
        store.increment_copy_count(&item.id).expect("copy once");
        store.increment_copy_count(&item.id).expect("copy twice");
        item.full_text = Some("remote".into());
        item.preview = "remote".into();
        item.sync_updated_at = "2999-01-01T00:00:00Z".into();
        item.sync_device_id = "remote-device".into();

        assert!(store.upsert_synced_item(&item).expect("upsert remote"));

        let stored = store.get_item(&item.id).expect("item").expect("row");
        assert_eq!(stored.full_text.as_deref(), Some("remote"));
        assert_eq!(stored.sync_device_id, "remote-device");
        assert_eq!(stored.copy_count, 2);

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
    fn auto_marks_sensitive_text_with_red_tag() {
        let paths = test_paths();
        let mut store = SqliteHistoryStore::new(&paths).expect("store");
        let settings = AppSettings::default();

        let inserted = store
            .upsert_capture(
                text_capture("tp-cbs7fccxc3qetc2axwabzgw9kah62xrsz9nxy0w6"),
                None,
                &settings,
            )
            .expect("insert sensitive text");

        assert_eq!(inserted.item.tag_colors, vec!["red".to_string()]);
        assert_eq!(
            inserted.item.full_text.as_deref(),
            Some("tp-cbs7fccxc3qetc2axwabzgw9kah62xrsz9nxy0w6")
        );

        let stored = store.list_all().expect("all").remove(0);
        assert_eq!(stored.tag_colors, vec!["red".to_string()]);

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
