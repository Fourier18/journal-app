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

use std::collections::{BTreeSet, HashMap, HashSet};

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
    /// forward links: source_id → set of target_ids this entry links to
    links: HashMap<String, HashSet<String>>,
    /// backward links: target_id → set of source_ids that link here
    backlinks: HashMap<String, HashSet<String>>,
}

impl MemIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.records.clear();
        self.links.clear();
        self.backlinks.clear();
    }

    /// Insert or replace a record (keyed by entry id).
    pub fn upsert(&mut self, record: Record) {
        let id = record.entry.id.clone();
        let new_targets = extract_links(&record.body);

        // Remove old forward links for this entry (and clean up backlinks).
        if let Some(old_targets) = self.links.remove(&id) {
            for t in &old_targets {
                if let Some(srcs) = self.backlinks.get_mut(t) {
                    srcs.remove(&id);
                }
            }
        }

        // Insert new forward links.
        let target_set: HashSet<String> = new_targets.into_iter().collect();
        for t in &target_set {
            self.backlinks.entry(t.clone()).or_default().insert(id.clone());
        }
        if !target_set.is_empty() {
            self.links.insert(id.clone(), target_set);
        }

        self.records.insert(id, record);
    }

    pub fn remove(&mut self, id: &str) -> Result<(), StorageError> {
        // Clean up forward links from this entry.
        if let Some(targets) = self.links.remove(id) {
            for t in &targets {
                if let Some(srcs) = self.backlinks.get_mut(t) {
                    srcs.remove(id);
                }
            }
        }
        // Clean up any backlinks that point from other entries to this one.
        self.backlinks.remove(id);

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

    /// Summaries of entries that contain a wikilink pointing *to* `id`.
    /// Newest first.
    pub fn backlinks_for(&self, id: &str) -> Vec<EntrySummary> {
        let source_ids = match self.backlinks.get(id) {
            Some(s) => s,
            None => return Vec::new(),
        };
        let mut out: Vec<EntrySummary> = source_ids
            .iter()
            .filter_map(|src| self.records.get(src))
            .map(summary_of)
            .collect();
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        out
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

/// Parse all `[[target-id]]` wikilink targets out of a body string.
fn extract_links(body: &str) -> Vec<String> {
    let mut links = Vec::new();
    let b = body.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'[' && b[i + 1] == b'[' {
            let rest = &body[i + 2..];
            if let Some(end) = rest.find("]]") {
                let target = &rest[..end];
                // Reject nested brackets or empty targets.
                if !target.is_empty() && !target.contains('[') && !target.contains(']') {
                    links.push(target.to_string());
                }
                i += 2 + end + 2;
                continue;
            }
        }
        i += 1;
    }
    links
}

fn entry_type_str(et: &EntryType) -> &'static str {
    match et {
        EntryType::Daily => "daily",
        EntryType::FreeForm => "free_form",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::entry::Entry;

    fn make_record(id: &str, body: &str) -> Record {
        let mut e = Entry::new_daily();
        e.id = id.to_string();
        Record { entry: e, file_path: format!("{}.md", id), body: body.to_string() }
    }

    #[test]
    fn extract_links_basic() {
        let links = extract_links("See also [[abc-123]] and [[def-456]].");
        assert_eq!(links, vec!["abc-123", "def-456"]);
    }

    #[test]
    fn extract_links_empty_and_nested() {
        assert!(extract_links("no links here").is_empty());
        assert!(extract_links("[[]]").is_empty());          // empty target
        assert!(extract_links("[[[bad]]]").is_empty());     // nested bracket
    }

    #[test]
    fn backlinks_populated_on_upsert() {
        let mut idx = MemIndex::new();
        idx.upsert(make_record("entry-a", ""));
        idx.upsert(make_record("entry-b", "Links to [[entry-a]]."));

        let bl = idx.backlinks_for("entry-a");
        assert_eq!(bl.len(), 1);
        assert_eq!(bl[0].id, "entry-b");
    }

    #[test]
    fn backlinks_cleared_on_remove() {
        let mut idx = MemIndex::new();
        idx.upsert(make_record("entry-a", ""));
        idx.upsert(make_record("entry-b", "[[entry-a]]"));

        idx.remove("entry-b").unwrap();
        assert!(idx.backlinks_for("entry-a").is_empty());
    }

    #[test]
    fn backlinks_updated_on_edit() {
        let mut idx = MemIndex::new();
        idx.upsert(make_record("entry-a", ""));
        idx.upsert(make_record("entry-b", ""));
        idx.upsert(make_record("link-src", "Points to [[entry-a]]."));

        // Edit link-src to point to entry-b instead.
        idx.upsert(make_record("link-src", "Points to [[entry-b]]."));

        assert!(idx.backlinks_for("entry-a").is_empty());
        assert_eq!(idx.backlinks_for("entry-b").len(), 1);
    }

    #[test]
    fn no_backlinks_for_unknown_id() {
        let idx = MemIndex::new();
        assert!(idx.backlinks_for("does-not-exist").is_empty());
    }
}
