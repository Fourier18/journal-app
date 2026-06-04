use chrono::Utc;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{
    crypto::{decrypt, encrypt, generate_salt, VaultKey},
    db::{MemIndex, Record},
    entry::{Entry, EntryType},
    StorageError,
};

const VERIFY_PLAINTEXT: &[u8] = b"journal-app-v1";

/// Lightweight summary of an entry for list/search results.
#[derive(Debug, Clone, Serialize)]
pub struct EntrySummary {
    pub id: String,
    pub file_path: String,
    pub entry_type: String,
    pub template: String,
    pub created_at: String,
    pub updated_at: String,
    pub title: Option<String>,
    pub tags: Vec<String>,
}

#[allow(dead_code)] // Locked variant used in Phase 3 when vault is held in AppState
enum VaultState {
    Locked,
    Unlocked { key: VaultKey, index: MemIndex },
}

pub struct Vault {
    root: PathBuf,
    state: VaultState,
}

impl Vault {
    // ── Lifecycle ──────────────────────────────────────────────────────────────

    /// First-run: create folder structure, salt, and verify blob.
    pub fn create(root: &Path, password: &str) -> Result<Self, StorageError> {
        let dot = root.join(".journal");
        fs::create_dir_all(&dot)?;
        fs::create_dir_all(root.join("entries"))?;
        fs::create_dir_all(root.join("attachments"))?;
        fs::create_dir_all(root.join("templates"))?;

        let salt = generate_salt();
        fs::write(dot.join("salt"), salt)?;

        let key = VaultKey::derive(password, &salt)?;

        let verify_blob = encrypt(VERIFY_PLAINTEXT, &key)?;
        fs::write(dot.join("verify"), verify_blob)?;

        fs::write(
            dot.join("config.toml"),
            "# Journal App configuration\nversion = 1\n",
        )?;

        Ok(Vault {
            root: root.to_path_buf(),
            state: VaultState::Unlocked {
                key,
                index: MemIndex::new(),
            },
        })
    }

    /// Subsequent runs: read salt, derive key, verify password, then load all
    /// entries into the in-memory index by decrypting the `.md` files.
    pub fn unlock(root: &Path, password: &str) -> Result<Self, StorageError> {
        let dot = root.join(".journal");
        let salt = fs::read(dot.join("salt"))?;
        let key = VaultKey::derive(password, &salt)?;

        let verify_blob = fs::read(dot.join("verify"))?;
        let check = decrypt(&verify_blob, &key)?;
        if check != VERIFY_PLAINTEXT {
            return Err(StorageError::WrongPassword);
        }

        // Purge any legacy plaintext SQLite index from older builds.
        purge_legacy_index(&dot);

        let mut vault = Vault {
            root: root.to_path_buf(),
            state: VaultState::Unlocked {
                key,
                index: MemIndex::new(),
            },
        };
        vault.load_all()?;
        Ok(vault)
    }

    /// Lock the vault — key is dropped and zeroized by ZeroizeOnDrop.
    pub fn lock(self) {}

    // ── CRUD ───────────────────────────────────────────────────────────────────

    pub fn create_entry(&mut self, entry: &Entry, body: &str) -> Result<(), StorageError> {
        let file_path = entry_file_path(
            &entry.entry_type,
            &entry.created_at,
            entry.title.as_deref(),
        );
        let root = self.root.clone();
        let abs_path = root.join(&file_path);
        fs::create_dir_all(abs_path.parent().unwrap())?;

        let content = entry.to_file_content(body)?;
        let (key, index) = self.unlocked_mut()?;
        let ciphertext = encrypt(content.as_bytes(), key)?;
        fs::write(&abs_path, ciphertext)?;
        index.upsert(Record {
            entry: entry.clone(),
            file_path,
            body: body.to_string(),
        });
        Ok(())
    }

    pub fn read_entry(&self, id: &str) -> Result<(Entry, String), StorageError> {
        // Content is already decrypted in memory — no file read needed.
        let (_, index) = self.unlocked_ref()?;
        let rec = index.get(id)?;
        Ok((rec.entry.clone(), rec.body.clone()))
    }

    pub fn update_entry(
        &mut self,
        id: &str,
        entry: &Entry,
        body: &str,
    ) -> Result<(), StorageError> {
        let root = self.root.clone();
        let file_path = {
            let (_, index) = self.unlocked_ref()?;
            index.get_file_path(id)?
        };
        let abs_path = root.join(&file_path);
        let content = entry.to_file_content(body)?;
        let (key, index) = self.unlocked_mut()?;
        let ciphertext = encrypt(content.as_bytes(), key)?;
        fs::write(&abs_path, ciphertext)?;
        index.upsert(Record {
            entry: entry.clone(),
            file_path,
            body: body.to_string(),
        });
        Ok(())
    }

    pub fn delete_entry(&mut self, id: &str) -> Result<(), StorageError> {
        let root = self.root.clone();
        let file_path = {
            let (_, index) = self.unlocked_ref()?;
            index.get_file_path(id)?
        };
        let abs_path = root.join(&file_path);
        if abs_path.exists() {
            fs::remove_file(&abs_path)?;
        }
        let (_, index) = self.unlocked_mut()?;
        index.remove(id)?;
        Ok(())
    }

    pub fn list_entries(&self) -> Result<Vec<EntrySummary>, StorageError> {
        let (_, index) = self.unlocked_ref()?;
        Ok(index.list())
    }

    /// Free-text + tag search over the in-memory index.
    pub fn search(&self, query: &str, tags: &[String]) -> Result<Vec<EntrySummary>, StorageError> {
        let (_, index) = self.unlocked_ref()?;
        Ok(index.search(query, tags))
    }

    /// Every distinct tag across all entries (sorted).
    pub fn all_tags(&self) -> Result<Vec<String>, StorageError> {
        let (_, index) = self.unlocked_ref()?;
        Ok(index.all_tags())
    }

    /// Entries that contain a wikilink pointing to `id` (newest first).
    pub fn get_backlinks(&self, id: &str) -> Result<Vec<EntrySummary>, StorageError> {
        let (_, index) = self.unlocked_ref()?;
        Ok(index.backlinks_for(id))
    }

    /// (Re)load the in-memory index by scanning and decrypting all entry files.
    /// This is the source of truth — the encrypted `.md` files — being read
    /// back into RAM. Called automatically on unlock.
    pub fn rebuild_index(&mut self) -> Result<usize, StorageError> {
        self.load_all()
    }

    fn load_all(&mut self) -> Result<usize, StorageError> {
        let entries_dir = self.root.join("entries");
        let root = self.root.clone();
        {
            let (_, index) = self.unlocked_mut()?;
            index.clear();
        }
        if !entries_dir.exists() {
            return Ok(0);
        }
        let mut md_files: Vec<PathBuf> = Vec::new();
        collect_md_files(&entries_dir, &mut md_files)?;

        let mut count = 0;
        for abs_path in md_files {
            let ciphertext = match fs::read(&abs_path) {
                Ok(b) => b,
                Err(_) => continue,
            };
            let (key, index) = self.unlocked_mut()?;
            let plaintext = match decrypt(&ciphertext, key) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let content = match String::from_utf8(plaintext) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let (entry, body) = match Entry::from_file_content(&content) {
                Ok(pair) => pair,
                Err(_) => continue,
            };
            let rel = abs_path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            index.upsert(Record {
                entry,
                file_path: rel,
                body,
            });
            count += 1;
        }
        Ok(count)
    }

    // ── Helpers ────────────────────────────────────────────────────────────────

    fn unlocked_ref(&self) -> Result<(&VaultKey, &MemIndex), StorageError> {
        match &self.state {
            VaultState::Unlocked { key, index } => Ok((key, index)),
            VaultState::Locked => Err(StorageError::Locked),
        }
    }

    fn unlocked_mut(&mut self) -> Result<(&VaultKey, &mut MemIndex), StorageError> {
        match &mut self.state {
            VaultState::Unlocked { key, index } => Ok((key, index)),
            VaultState::Locked => Err(StorageError::Locked),
        }
    }
}

/// Remove any leftover plaintext SQLite index from older builds (and its WAL).
fn purge_legacy_index(dot: &Path) {
    for name in ["index.db", "index.db-wal", "index.db-shm"] {
        let _ = fs::remove_file(dot.join(name));
    }
}

fn entry_file_path(
    entry_type: &EntryType,
    created_at: &chrono::DateTime<Utc>,
    title: Option<&str>,
) -> String {
    let date = created_at.format("%Y-%m-%d").to_string();
    let year = created_at.format("%Y").to_string();
    let month = created_at.format("%m").to_string();
    let filename = match entry_type {
        EntryType::Daily => format!("{}.md", date),
        EntryType::FreeForm => {
            let slug = title
                .map(slugify)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()[..8].to_string());
            format!("{}-{}.md", date, slug)
        }
    };
    format!("entries/{}/{}/{}", year, month, filename)
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(40)
        .collect()
}

fn collect_md_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), StorageError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_vault(dir: &TempDir) -> Vault {
        Vault::create(dir.path(), "test-password-123").unwrap()
    }

    #[test]
    fn create_and_unlock() {
        let dir = TempDir::new().unwrap();
        let _v = make_vault(&dir);
        drop(_v);

        let vault2 = Vault::unlock(dir.path(), "test-password-123").unwrap();
        assert!(vault2.list_entries().unwrap().is_empty());
    }

    #[test]
    fn wrong_password_rejected() {
        let dir = TempDir::new().unwrap();
        let _v = make_vault(&dir);
        drop(_v);

        let result = Vault::unlock(dir.path(), "wrong-password");
        assert!(matches!(result, Err(StorageError::WrongPassword)));
    }

    #[test]
    fn create_read_entry() {
        let dir = TempDir::new().unwrap();
        let mut vault = make_vault(&dir);

        let entry = Entry::new_daily();
        let body = "Today was a great day.";
        vault.create_entry(&entry, body).unwrap();

        let (read_entry, read_body) = vault.read_entry(&entry.id).unwrap();
        assert_eq!(entry.id, read_entry.id);
        assert_eq!(body, read_body);
    }

    #[test]
    fn create_update_read_entry() {
        let dir = TempDir::new().unwrap();
        let mut vault = make_vault(&dir);

        let mut entry = Entry::new_daily();
        vault.create_entry(&entry, "initial body").unwrap();

        entry.updated_at = Utc::now();
        entry.tags.push("updated".into());
        vault
            .update_entry(&entry.id.clone(), &entry, "updated body")
            .unwrap();

        let (read_entry, read_body) = vault.read_entry(&entry.id).unwrap();
        assert_eq!(read_body, "updated body");
        assert!(read_entry.tags.contains(&"updated".to_string()));
    }

    #[test]
    fn delete_entry() {
        let dir = TempDir::new().unwrap();
        let mut vault = make_vault(&dir);

        let entry = Entry::new_daily();
        vault.create_entry(&entry, "to be deleted").unwrap();

        vault.delete_entry(&entry.id).unwrap();
        assert!(vault.list_entries().unwrap().is_empty());
    }

    #[test]
    fn unlock_loads_existing_entries() {
        let dir = TempDir::new().unwrap();
        let mut vault = make_vault(&dir);

        let e1 = Entry::new_daily();
        let e2 = Entry::new_free_form(Some("My thoughts".into()));
        vault.create_entry(&e1, "day one").unwrap();
        vault.create_entry(&e2, "free form body").unwrap();
        drop(vault);

        // Unlock rebuilds the in-memory index from the encrypted files.
        let vault2 = Vault::unlock(dir.path(), "test-password-123").unwrap();
        assert_eq!(vault2.list_entries().unwrap().len(), 2);
    }

    #[test]
    fn rebuild_index() {
        let dir = TempDir::new().unwrap();
        let mut vault = make_vault(&dir);

        vault.create_entry(&Entry::new_daily(), "day one").unwrap();
        vault
            .create_entry(&Entry::new_free_form(Some("My thoughts".into())), "body")
            .unwrap();

        let count = vault.rebuild_index().unwrap();
        assert_eq!(count, 2);
        assert_eq!(vault.list_entries().unwrap().len(), 2);
    }

    #[test]
    fn list_entries_returns_all() {
        let dir = TempDir::new().unwrap();
        let mut vault = make_vault(&dir);

        for _ in 0..3 {
            vault.create_entry(&Entry::new_daily(), "body").unwrap();
        }
        assert_eq!(vault.list_entries().unwrap().len(), 3);
    }

    #[test]
    fn search_by_body_text() {
        let dir = TempDir::new().unwrap();
        let mut vault = make_vault(&dir);

        vault
            .create_entry(&Entry::new_daily(), "I went hiking in the mountains")
            .unwrap();
        vault
            .create_entry(&Entry::new_daily(), "Quiet day reading at home")
            .unwrap();

        let hits = vault.search("HIKING", &[]).unwrap(); // case-insensitive
        assert_eq!(hits.len(), 1);

        let none = vault.search("submarine", &[]).unwrap();
        assert!(none.is_empty());

        let all = vault.search("", &[]).unwrap(); // empty query = all
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn search_by_title() {
        let dir = TempDir::new().unwrap();
        let mut vault = make_vault(&dir);

        vault
            .create_entry(&Entry::new_free_form(Some("Dream Journal".into())), "body")
            .unwrap();
        vault.create_entry(&Entry::new_daily(), "ordinary").unwrap();

        let hits = vault.search("dream", &[]).unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn search_and_filter_by_tags() {
        let dir = TempDir::new().unwrap();
        let mut vault = make_vault(&dir);

        let mut e1 = Entry::new_daily();
        e1.tags = vec!["work".into(), "urgent".into()];
        let mut e2 = Entry::new_daily();
        e2.tags = vec!["work".into()];
        let e3 = Entry::new_daily();
        vault.create_entry(&e1, "alpha").unwrap();
        vault.create_entry(&e2, "beta").unwrap();
        vault.create_entry(&e3, "gamma").unwrap();

        assert_eq!(vault.search("", &["work".into()]).unwrap().len(), 2);
        assert_eq!(
            vault
                .search("", &["work".into(), "urgent".into()])
                .unwrap()
                .len(),
            1
        );
        // text + tag combined
        assert_eq!(
            vault.search("beta", &["work".into()]).unwrap().len(),
            1
        );

        let mut tags = vault.all_tags().unwrap();
        tags.sort();
        assert_eq!(tags, vec!["urgent".to_string(), "work".to_string()]);
    }

    /// Privacy guard: creating an entry must NOT produce a plaintext index file.
    /// The encrypted `.md` files are the only on-disk copy of content.
    #[test]
    fn no_plaintext_index_on_disk() {
        let dir = TempDir::new().unwrap();
        let mut vault = make_vault(&dir);
        vault
            .create_entry(&Entry::new_daily(), "a secret confession")
            .unwrap();

        let dot = dir.path().join(".journal");
        assert!(!dot.join("index.db").exists(), "index.db must not exist");
        assert!(!dot.join("index.db-wal").exists());

        // And the secret must not appear in cleartext anywhere under .journal/.
        for entry in fs::read_dir(&dot).unwrap() {
            let p = entry.unwrap().path();
            if p.is_file() {
                let bytes = fs::read(&p).unwrap();
                assert!(
                    !bytes
                        .windows(b"secret confession".len())
                        .any(|w| w == b"secret confession"),
                    "plaintext leaked into {:?}",
                    p
                );
            }
        }
    }
}
