//! In-memory entry index.
//!
//! This REPLACES the previous SQLite index. The old index stored decrypted
//! entry bodies and metadata in a plain (unencrypted) `index.db` file, which
//! defeated the whole point of encrypting the `.md` files — anyone with the
//! index file could read every entry without the password.
//!
//! The encrypted `.md` files are now the only on-disk copy of your content.
//! This index lives only in RAM while the vault is unlocked, is rebuilt from
//! the encrypted files at unlock time, and is dropped (along with the vault
//! key) when you lock. No journal content is ever written to disk unencrypted.

use std::collections::{BTreeSet, HashMap};

use super::{
    entry::{Entry, EntryType},
    vault::EntrySummary,
    StorageError,
};

/// One fully-loaded entry, held in memory only.
pub struct Record {
    pub entry: Entry,
    pub file_path: String,
    pub body: String,
}

/// In-memory index of all entries. Nothing here is persisted.
#[derive(Default)]
pub struct MemIndex {
    records: HashMap<String, Record>,
}

impl MemIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Insert or replace a record (keyed by entry id).
    pub fn upsert(&mut self, record: Record) {
        self.records.insert(record.entry.id.clone(), record);
    }

    pub fn remove(&mut self, id: &str) -> Result<(), StorageError> {
        self.records
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| StorageError::NotFound(id.to_string()))
    }

    pub fn exists(&self, id: &str) -> bool {
        self.records.contains_key(id)
    }

    pub fn get(&self, id: &str) -> Result<&Record, StorageError> {
        self.records
            .get(id)
            .ok_or_else(|| StorageError::NotFound(id.to_string()))
    }

    pub fn get_file_path(&self, id: &str) -> Result<String, StorageError> {
        self.get(id).map(|r| r.file_path.clone())
    }

    /// All entries as summaries, newest first.
    pub fn list(&self) -> Vec<EntrySummary> {
        let mut out: Vec<EntrySummary> = self.records.values().map(summary_of).collect();
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        out
    }

    /// Entries matching a free-text query (in body or title) AND containing
    /// every tag in `tags`. Empty query matches all; empty tags matches all.
    /// Case-insensitive throughout. Newest first.
    pub fn search(&self, query: &str, tags: &[String]) -> Vec<EntrySummary> {
        let q = query.trim().to_lowercase();
        let mut out: Vec<EntrySummary> = self
            .records
            .values()
            .filter(|r| {
                let text_ok = q.is_empty()
                    || r.body.to_lowercase().contains(&q)
                    || r
                        .entry
                        .title
                        .as_deref()
                        .map(|t| t.to_lowercase().contains(&q))
                        .unwrap_or(false);

                let tags_ok = tags.iter().all(|wanted| {
                    r.entry
                        .tags
                        .iter()
                        .any(|have| have.eq_ignore_ascii_case(wanted))
                });

                text_ok && tags_ok
            })
            .map(summary_of)
            .collect();
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        out
    }

    /// Sorted, de-duplicated set of every tag across all entries.
    pub fn all_tags(&self) -> Vec<String> {
        let mut set: BTreeSet<String> = BTreeSet::new();
        for r in self.records.values() {
            for t in &r.entry.tags {
                set.insert(t.clone());
            }
        }
        set.into_iter().collect()
    }
}

fn summary_of(r: &Record) -> EntrySummary {
    EntrySummary {
        id: r.entry.id.clone(),
        file_path: r.file_path.clone(),
        entry_type: entry_type_str(&r.entry.entry_type).to_string(),
        template: r.entry.template.clone(),
        created_at: r.entry.created_at.to_rfc3339(),
        updated_at: r.entry.updated_at.to_rfc3339(),
        title: r.entry.title.clone(),
        tags: r.entry.tags.clone(),
    }
}

fn entry_type_str(et: &EntryType) -> &'static str {
    match et {
        EntryType::Daily => "daily",
        EntryType::FreeForm => "free_form",
    }
}
