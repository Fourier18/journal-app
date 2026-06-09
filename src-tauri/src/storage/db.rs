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
use serde::{Deserialize, Serialize};

use super::{
    entry::{Entry, EntryType, MetadataValue},
    vault::EntrySummary,
    StorageError,
};

// ── Search types ──────────────────────────────────────────────────────────────

/// Options controlling what `MemIndex::search` looks at and how it ranks.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchOptions {
    pub query: String,
    /// Include body text in matches.
    pub in_body: bool,
    /// Include the entry title in matches.
    pub in_title: bool,
    /// Include tags in matches.
    pub in_tags: bool,
    /// Include metadata values in matches.
    pub in_metadata: bool,
    /// `true` → every whitespace-separated word must appear somewhere in the
    /// enabled fields (AND logic). `false` → the whole query string is treated
    /// as a single exact phrase.
    pub match_all_words: bool,
    /// `true` → sort by relevance score descending; `false` → newest first.
    pub sort_by_relevance: bool,
}

/// One segment of a result snippet — either plain text or a highlighted match.
#[derive(Debug, Clone, Serialize)]
pub struct SnippetSegment {
    pub text: String,
    /// `true` means this segment matches a query term and should be highlighted.
    pub hit: bool,
}

/// A pre-segmented excerpt from the entry body suitable for rendering with
/// `<mark>` highlights — no byte-offset math needed in the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct Snippet {
    pub segments: Vec<SnippetSegment>,
}

/// One result from `MemIndex::search`.
#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub entry: EntrySummary,
    pub snippet: Option<Snippet>,
    pub score: f64,
    /// The highest-weighted field that produced a match ("title", "tags",
    /// "metadata", or "body").
    pub matched_field: String,
}

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

    /// Options-driven search over the in-memory index.
    ///
    /// Empty query → returns all entries (newest first, no snippets).
    /// Otherwise matches by enabled scopes, scores by field weight, and
    /// returns `SearchHit`s with pre-segmented snippets.
    pub fn search(&self, opts: &SearchOptions) -> Vec<SearchHit> {
        let raw_query = opts.query.trim();

        // Empty query — return everything newest-first, no snippets.
        if raw_query.is_empty() {
            let mut out: Vec<SearchHit> = self
                .records
                .values()
                .map(|r| SearchHit {
                    entry: summary_of(r),
                    snippet: None,
                    score: 0.0,
                    matched_field: String::new(),
                })
                .collect();
            out.sort_by(|a, b| b.entry.created_at.cmp(&a.entry.created_at));
            return out;
        }

        // Tokenize: all-words → split on whitespace; exact-phrase → one term.
        let terms: Vec<Vec<char>> = if opts.match_all_words {
            raw_query
                .split_whitespace()
                .map(|w| w.to_lowercase().chars().collect())
                .collect()
        } else {
            vec![raw_query.to_lowercase().chars().collect()]
        };

        let mut hits: Vec<SearchHit> = Vec::new();

        for r in self.records.values() {
            // Build char-array versions of each searchable field (lowercased).
            let title_chars: Vec<char> = r
                .entry
                .title
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .chars()
                .collect();
            let body_chars: Vec<char> =
                r.body.to_lowercase().chars().collect();
            let tags_joined = r.entry.tags.join(" ").to_lowercase();
            let tags_chars: Vec<char> = tags_joined.chars().collect();
            let meta_joined = if opts.in_metadata {
                r.entry
                    .metadata
                    .values()
                    .map(|v| match v {
                        MetadataValue::Number(n) => n.to_string(),
                        MetadataValue::Text(s) => s.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase()
            } else {
                String::new()
            };
            let meta_chars: Vec<char> = meta_joined.chars().collect();

            // Check which scopes match all terms.
            let title_match = opts.in_title
                && !title_chars.is_empty()
                && terms.iter().all(|t| char_contains(&title_chars, t));
            let body_match = opts.in_body
                && !body_chars.is_empty()
                && terms.iter().all(|t| char_contains(&body_chars, t));
            let tags_match = opts.in_tags
                && !tags_chars.is_empty()
                && terms.iter().all(|t| char_contains(&tags_chars, t));
            let meta_match = opts.in_metadata
                && !meta_chars.is_empty()
                && terms.iter().all(|t| char_contains(&meta_chars, t));

            if !title_match && !body_match && !tags_match && !meta_match {
                continue;
            }

            // Score: title ×5, tags/metadata ×3, body ×1.
            let mut score = 0.0f64;
            let mut best_field = String::new();
            let mut best_weight = 0.0f64;

            let mut add_field = |name: &str, weight: f64, chars: &[char]| {
                let w = weight
                    * terms
                        .iter()
                        .map(|t| count_char_occurrences(chars, t) as f64)
                        .sum::<f64>();
                score += w;
                if w > best_weight {
                    best_weight = w;
                    best_field = name.to_string();
                }
            };

            if title_match { add_field("title",    5.0, &title_chars); }
            if tags_match  { add_field("tags",     3.0, &tags_chars);  }
            if meta_match  { add_field("metadata", 3.0, &meta_chars);  }
            if body_match  { add_field("body",     1.0, &body_chars);  }

            // Snippet: prefer body for context, fall back to best matched field.
            let snippet = if body_match {
                build_snippet(&r.body, &terms)
            } else if title_match {
                build_snippet(r.entry.title.as_deref().unwrap_or(""), &terms)
            } else if tags_match {
                build_snippet(&r.entry.tags.join(" "), &terms)
            } else {
                build_snippet(&meta_joined, &terms)
            };

            hits.push(SearchHit {
                entry: summary_of(r),
                snippet,
                score,
                matched_field: best_field,
            });
        }

        // Sort: relevance desc (tie-break newest), or newest-first.
        if opts.sort_by_relevance {
            hits.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(b.entry.created_at.cmp(&a.entry.created_at))
            });
        } else {
            hits.sort_by(|a, b| b.entry.created_at.cmp(&a.entry.created_at));
        }

        hits
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

// ── Search helpers ────────────────────────────────────────────────────────────

/// Returns `true` if `haystack` contains `needle` as a contiguous char slice.
fn char_contains(haystack: &[char], needle: &[char]) -> bool {
    if needle.is_empty() { return true; }
    if needle.len() > haystack.len() { return false; }
    haystack
        .windows(needle.len())
        .any(|w| w == needle)
}

/// Count non-overlapping occurrences of `needle` in `haystack` (char slices).
fn count_char_occurrences(haystack: &[char], needle: &[char]) -> usize {
    if needle.is_empty() { return 0; }
    let mut count = 0;
    let mut i = 0;
    while i + needle.len() <= haystack.len() {
        if &haystack[i..i + needle.len()] == needle {
            count += 1;
            i += needle.len();
        } else {
            i += 1;
        }
    }
    count
}

/// Build a pre-segmented ~120-char snippet from `source` around the first
/// occurrence of any term. Uses char-level indexing — UTF-8 safe throughout.
fn build_snippet(source: &str, terms: &[Vec<char>]) -> Option<Snippet> {
    if source.is_empty() || terms.is_empty() {
        return None;
    }

    let src_chars: Vec<char> = source.chars().collect();
    let low_chars: Vec<char> = src_chars
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();

    // Find the earliest match of any term.
    let first_pos = terms
        .iter()
        .filter_map(|t| {
            if t.is_empty() { return None; }
            low_chars
                .windows(t.len())
                .position(|w| w == t.as_slice())
        })
        .min()?;

    let total = src_chars.len();
    let half = 60usize;
    let start = first_pos.saturating_sub(half);
    let end = (first_pos + half + 1).min(total);

    let win_src = &src_chars[start..end];
    let win_low = &low_chars[start..end];
    let win_len = win_src.len();

    // Build a hit-mask: mark every char that belongs to a term match.
    let mut hit_mask = vec![false; win_len];
    for term in terms {
        let tlen = term.len();
        if tlen == 0 { continue; }
        let mut i = 0;
        while i + tlen <= win_len {
            if &win_low[i..i + tlen] == term.as_slice() {
                for j in i..i + tlen {
                    hit_mask[j] = true;
                }
                i += tlen;
            } else {
                i += 1;
            }
        }
    }

    // Build segments from contiguous same-type runs.
    let mut segments: Vec<SnippetSegment> = Vec::new();
    let mut i = 0;
    while i < win_len {
        let is_hit = hit_mask[i];
        let mut j = i + 1;
        while j < win_len && hit_mask[j] == is_hit {
            j += 1;
        }
        segments.push(SnippetSegment {
            text: win_src[i..j].iter().collect(),
            hit: is_hit,
        });
        i = j;
    }

    // Prepend/append ellipsis if we're not at the edge of the source.
    if start > 0 {
        segments.insert(0, SnippetSegment { text: "…".to_string(), hit: false });
    }
    if end < total {
        segments.push(SnippetSegment { text: "…".to_string(), hit: false });
    }

    if segments.is_empty() { None } else { Some(Snippet { segments }) }
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

    // ── Search tests ──────────────────────────────────────────────────────────

    fn default_opts() -> SearchOptions {
        SearchOptions {
            query: String::new(),
            in_body: true,
            in_title: true,
            in_tags: true,
            in_metadata: true,
            match_all_words: true,
            sort_by_relevance: true,
        }
    }

    fn make_record_full(id: &str, title: Option<&str>, body: &str, tags: Vec<&str>) -> Record {
        let mut e = Entry::new_daily();
        e.id = id.to_string();
        e.title = title.map(|s| s.to_string());
        e.tags = tags.into_iter().map(|s| s.to_string()).collect();
        Record { entry: e, file_path: format!("{}.md", id), body: body.to_string() }
    }

    #[test]
    fn search_multi_word_and() {
        let mut idx = MemIndex::new();
        idx.upsert(make_record_full("a", None, "I went hiking in the mountains", vec![]));
        idx.upsert(make_record_full("b", None, "hiking was great but no mountains", vec![]));
        idx.upsert(make_record_full("c", None, "quiet day at home", vec![]));

        let opts = SearchOptions { query: "hiking mountains".into(), ..default_opts() };
        let hits: Vec<_> = idx.search(&opts);
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().any(|h| h.entry.id == "a"));
        assert!(hits.iter().any(|h| h.entry.id == "b"));
    }

    #[test]
    fn search_exact_phrase_vs_all_words() {
        let mut idx = MemIndex::new();
        idx.upsert(make_record_full("a", None, "mountain hiking trail", vec![]));
        idx.upsert(make_record_full("b", None, "hiking in the mountains", vec![]));

        // Exact phrase "mountain hiking" only matches entry a.
        let exact = SearchOptions {
            query: "mountain hiking".into(),
            match_all_words: false,
            ..default_opts()
        };
        let hits = idx.search(&exact);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entry.id, "a");

        // All-words matches both.
        let all = SearchOptions {
            query: "mountain hiking".into(),
            match_all_words: true,
            ..default_opts()
        };
        assert_eq!(idx.search(&all).len(), 2);
    }

    #[test]
    fn search_title_only_scope() {
        let mut idx = MemIndex::new();
        idx.upsert(make_record_full("a", Some("Dream Journal"), "ordinary body", vec![]));
        idx.upsert(make_record_full("b", None, "I had a vivid dream last night", vec![]));

        let opts = SearchOptions {
            query: "dream".into(),
            in_body: false,
            in_tags: false,
            in_metadata: false,
            ..default_opts()
        };
        let hits = idx.search(&opts);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entry.id, "a");
        assert_eq!(hits[0].matched_field, "title");
    }

    #[test]
    fn search_tags_scope() {
        let mut idx = MemIndex::new();
        idx.upsert(make_record_full("a", None, "some body", vec!["work", "urgent"]));
        idx.upsert(make_record_full("b", None, "work in the body text", vec![]));

        let opts = SearchOptions {
            query: "work".into(),
            in_body: false,
            in_title: false,
            in_metadata: false,
            ..default_opts()
        };
        let hits = idx.search(&opts);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entry.id, "a");
        assert_eq!(hits[0].matched_field, "tags");
    }

    #[test]
    fn search_metadata_scope() {
        use crate::storage::entry::MetadataValue;
        let mut idx = MemIndex::new();
        let mut r = make_record_full("a", None, "today was fine", vec![]);
        r.entry.metadata.insert("weather".into(), MetadataValue::Text("rainy".into()));
        idx.upsert(r);
        idx.upsert(make_record_full("b", None, "another day", vec![]));

        let opts = SearchOptions { query: "rainy".into(), in_body: false, in_title: false, in_tags: false, ..default_opts() };
        let hits = idx.search(&opts);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entry.id, "a");
        assert_eq!(hits[0].matched_field, "metadata");
    }

    #[test]
    fn search_relevance_title_beats_body() {
        let mut idx = MemIndex::new();
        // "a" has the term in title (weight 5); "b" has it only in body (weight 1).
        idx.upsert(make_record_full("a", Some("Sunrise walk"), "went for a short stroll", vec![]));
        idx.upsert(make_record_full("b", None, "watched the sunrise from the porch", vec![]));

        let opts = SearchOptions { query: "sunrise".into(), sort_by_relevance: true, ..default_opts() };
        let hits = idx.search(&opts);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].entry.id, "a"); // title hit should rank first
        assert!(hits[0].score > hits[1].score);
    }

    #[test]
    fn search_snippet_contains_term() {
        let mut idx = MemIndex::new();
        idx.upsert(make_record_full("a", None, "The quick brown fox jumps over the lazy dog and then it went hiking up a very tall mountain peak on a clear sunny day", vec![]));

        let opts = SearchOptions { query: "hiking".into(), ..default_opts() };
        let hits = idx.search(&opts);
        assert_eq!(hits.len(), 1);
        let snippet = hits[0].snippet.as_ref().expect("should have snippet");
        let hit_segs: Vec<_> = snippet.segments.iter().filter(|s| s.hit).collect();
        assert!(!hit_segs.is_empty());
        assert_eq!(hit_segs[0].text.to_lowercase(), "hiking");
    }
}
