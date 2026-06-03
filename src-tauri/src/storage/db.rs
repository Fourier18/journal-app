use rusqlite::{params, Connection};
use std::path::Path;

use super::{
    entry::{Entry, EntryType, MetadataValue},
    StorageError,
};

pub struct JournalDb {
    pub conn: Connection,
}

impl JournalDb {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
        let db = JournalDb { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<(), StorageError> {
        self.conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS entries (
                id          TEXT PRIMARY KEY,
                file_path   TEXT NOT NULL,
                entry_type  TEXT NOT NULL,
                template    TEXT NOT NULL DEFAULT 'blank',
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL,
                title       TEXT
            );

            CREATE TABLE IF NOT EXISTS tags (
                entry_id TEXT NOT NULL,
                tag      TEXT NOT NULL,
                PRIMARY KEY (entry_id, tag),
                FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS metadata (
                entry_id   TEXT NOT NULL,
                key        TEXT NOT NULL,
                value_num  REAL,
                value_text TEXT,
                PRIMARY KEY (entry_id, key),
                FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS entry_fts USING fts5(
                entry_id UNINDEXED,
                body
            );
            "#,
        )?;
        Ok(())
    }

    pub fn insert_entry(
        &self,
        entry: &Entry,
        file_path: &str,
        body: &str,
    ) -> Result<(), StorageError> {
        let type_str = entry_type_str(&entry.entry_type);
        self.conn.execute(
            "INSERT INTO entries (id, file_path, entry_type, template, created_at, updated_at, title)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.id,
                file_path,
                type_str,
                entry.template,
                entry.created_at.to_rfc3339(),
                entry.updated_at.to_rfc3339(),
                entry.title,
            ],
        )?;
        self.upsert_tags(&entry.id, &entry.tags)?;
        self.upsert_metadata(&entry.id, &entry.metadata)?;
        self.conn.execute(
            "INSERT INTO entry_fts (entry_id, body) VALUES (?1, ?2)",
            params![entry.id, body],
        )?;
        Ok(())
    }

    pub fn update_entry(
        &self,
        entry: &Entry,
        file_path: &str,
        body: &str,
    ) -> Result<(), StorageError> {
        let type_str = entry_type_str(&entry.entry_type);
        let rows = self.conn.execute(
            "UPDATE entries SET file_path=?2, entry_type=?3, template=?4,
             updated_at=?5, title=?6 WHERE id=?1",
            params![
                entry.id,
                file_path,
                type_str,
                entry.template,
                entry.updated_at.to_rfc3339(),
                entry.title,
            ],
        )?;
        if rows == 0 {
            return Err(StorageError::NotFound(entry.id.clone()));
        }
        // Replace tags and metadata
        self.conn
            .execute("DELETE FROM tags WHERE entry_id = ?1", params![entry.id])?;
        self.conn
            .execute("DELETE FROM metadata WHERE entry_id = ?1", params![entry.id])?;
        self.conn
            .execute("DELETE FROM entry_fts WHERE entry_id = ?1", params![entry.id])?;
        self.upsert_tags(&entry.id, &entry.tags)?;
        self.upsert_metadata(&entry.id, &entry.metadata)?;
        self.conn.execute(
            "INSERT INTO entry_fts (entry_id, body) VALUES (?1, ?2)",
            params![entry.id, body],
        )?;
        Ok(())
    }

    pub fn delete_entry(&self, id: &str) -> Result<(), StorageError> {
        let rows = self
            .conn
            .execute("DELETE FROM entries WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(StorageError::NotFound(id.to_string()));
        }
        self.conn
            .execute("DELETE FROM entry_fts WHERE entry_id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_file_path(&self, id: &str) -> Result<String, StorageError> {
        self.conn
            .query_row(
                "SELECT file_path FROM entries WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|_| StorageError::NotFound(id.to_string()))
    }

    pub fn list_entries(&self) -> Result<Vec<crate::storage::vault::EntrySummary>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_path, entry_type, template, created_at, updated_at, title
             FROM entries ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::storage::vault::EntrySummary {
                id: row.get(0)?,
                file_path: row.get(1)?,
                entry_type: row.get(2)?,
                template: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                title: row.get(6)?,
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn entry_exists(&self, id: &str) -> Result<bool, StorageError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM entries WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    fn upsert_tags(&self, entry_id: &str, tags: &[String]) -> Result<(), StorageError> {
        for tag in tags {
            self.conn.execute(
                "INSERT OR IGNORE INTO tags (entry_id, tag) VALUES (?1, ?2)",
                params![entry_id, tag],
            )?;
        }
        Ok(())
    }

    fn upsert_metadata(
        &self,
        entry_id: &str,
        metadata: &std::collections::HashMap<String, MetadataValue>,
    ) -> Result<(), StorageError> {
        for (key, value) in metadata {
            match value {
                MetadataValue::Number(n) => self.conn.execute(
                    "INSERT OR REPLACE INTO metadata (entry_id, key, value_num, value_text)
                     VALUES (?1, ?2, ?3, NULL)",
                    params![entry_id, key, n],
                )?,
                MetadataValue::Text(t) => self.conn.execute(
                    "INSERT OR REPLACE INTO metadata (entry_id, key, value_num, value_text)
                     VALUES (?1, ?2, NULL, ?3)",
                    params![entry_id, key, t],
                )?,
            };
        }
        Ok(())
    }
}

fn entry_type_str(et: &EntryType) -> &'static str {
    match et {
        EntryType::Daily => "daily",
        EntryType::FreeForm => "free_form",
    }
}
