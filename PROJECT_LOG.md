# Project Log — Journal App

A running record of what was built, when, and why. Most recent phase at the top.

---

## Phase 6 — Wikilinks and backlinks
**Commit:** (see git log)

### What was built

#### Wikilink-style inter-entry references
- Type `[[` anywhere in the editor to open an autocomplete dropdown of all your entries (searched by title or date).
- Selecting an entry inserts `[[uuid]]` into the body text — the UUID is the stable identity, so links survive renames.
- Inserted wikilinks are **rendered inline as clickable title labels** — the raw UUID is hidden. Clicking navigates directly to the linked entry.
- If the linked entry is later deleted, the widget displays with a red strikethrough style ("broken link") rather than silently disappearing.

#### Backlinks panel
- A "Linked from (N)" panel appears at the bottom of the editor whenever another entry contains a wikilink pointing to the current one.
- Each item is clickable, navigating to the source entry.
- The panel is invisible when there are no backlinks (no empty-state clutter).

### Architecture changes

**Rust — `db.rs`:**
- `MemIndex` now carries two new maps: `links` (source → set of targets) and `backlinks` (target → set of sources).
- `upsert` parses `[[id]]` patterns from the body on every write and keeps both maps current.
- `remove` cleans up all forward and backward link entries for the deleted record.
- `clear()` also wipes the link maps — so `rebuild_index` starts clean.
- New method: `backlinks_for(id) → Vec<EntrySummary>`.
- Because `load_all` already calls `upsert` for every entry, the link index is fully populated on every vault unlock at no extra cost.

**Rust — `vault.rs`:**
- New method: `get_backlinks(id)` — thin wrapper around `MemIndex::backlinks_for`.

**Rust — `commands.rs` / `lib.rs`:**
- New Tauri command: `get_backlinks(id)` → registered in `invoke_handler`.

**TypeScript — `commands.ts`:**
- `getBacklinks(id)` wrapper.

**React:**
- `src/lib/wikilinkExtension.ts` — CodeMirror 6 extension that provides both the `[[` autocomplete and the replace-decoration rendering. Autocomplete uses `autocompletion({ override: [...] })`; decorations use `MatchDecorator` + `Decoration.replace` widgets. Wikilinks are atomic (cursor can't enter the UUID text). Extension is rebuilt via `useMemo` when the entries list changes.
- `src/components/BacklinksPanel.tsx` + `.css` — loads backlinks on `entryId` change, renders the linked-from list.
- `Editor.tsx` — imports and mounts both; `autocompletion: false` in basicSetup to avoid conflicts.

### Tag input and tag filtering — already done
Both features were completed in Phases 4 and 5 respectively — no duplicate work was needed.

### Test count: 24 passing (up from 18)
6 new tests: `extract_links_basic`, `extract_links_empty_and_nested`, `backlinks_populated_on_upsert`, `backlinks_cleared_on_remove`, `backlinks_updated_on_edit`, `no_backlinks_for_unknown_id`

---

## Phase 5 — Full-text search, tag filtering, and privacy fix
**Commit:** `c08ad7e`

### What was built
- Full-text search across entry bodies and titles — case-insensitive, debounced 200ms, live results in the sidebar
- Tag filter chips in the sidebar — toggle any number of tags; results AND-filter (entry must have all selected tags)
- Clear button dismisses both query and tag filters at once
- "No entries match" empty state when filters return nothing

### Architecture change — plaintext index removed
The original storage layer used rusqlite (SQLite) with an FTS5 full-text index. During this phase we confirmed that `index.db` and `index.db-wal` stored **decrypted entry text on disk in plaintext** — readable without the password. This directly undercut the encryption.

**Fix:** removed rusqlite entirely. The index is now in-memory only (`MemIndex` in `db.rs`), built by decrypting the `.md` files at unlock and held in RAM alongside the vault key. It is dropped when you lock. Nothing decrypted is ever written to disk.

`unlock()` now calls `purge_legacy_index()` on every run, which silently deletes any leftover `index.db`/`-wal`/`-shm` files from older vaults.

A dedicated test (`no_plaintext_index_on_disk`) was added to the suite as a regression guard — it fails if any plaintext index file ever reappears.

### Test count: 18 passing (up from 13)
5 new tests: `unlock_loads_existing_entries`, `search_by_body_text`, `search_by_title`, `search_and_filter_by_tags`, `no_plaintext_index_on_disk`

---

## Phase 4 — Templates, tags, metadata, and entry management
**Commit:** `e5874dd`

### What was built
- **6 writing templates:** Daily Entry, Gratitude, Morning Pages, Weekly Review, Work Log, Free-form
- **New Entry modal:** choose entry type and template before creating
- **Tags UI:** add/remove tags inline on any entry; tag chips display in the sidebar entry list
- **Metadata panel:** structured fields alongside freeform writing — mood (1–10 slider), sleep hours, and open-ended custom fields
- **Editable titles:** click to rename any entry; defaults to the date if left blank
- **Delete with confirm:** trash icon on each entry, confirm dialog before removal

---

## Phase 3 — Editor, lock screen, and design system
**Commit:** `ec0cfaf`

### What was built
- **CodeMirror 6 editor:** Markdown mode with live syntax hints (not a preview — the document stays as text), auto-save on change
- **Three themes:** Light, Dark, Sepia — toggle cycles through them; choice persists across sessions
- **Lock screen:** first-run vault setup (choose master password) and returning unlock flow; wrong password shows an error, no recovery
- **CSS design tokens:** all colours, spacing, and typography go through CSS custom properties — makes theming a single-file change
- **Tauri command bridge:** all Rust ↔ React communication wired through typed TypeScript wrappers in `src/lib/commands.ts`

---

## Phase 2 — Storage and encryption layer
**Commit:** `0dffa53`

### What was built
- **Vault creation:** generates an argon2id salt, derives a key from the master password (64 MiB / 3 iterations), writes a small `verify` blob to confirm the password on future unlocks
- **age encryption:** every entry file is encrypted whole — body, tags, and metadata — using the derived key as the age passphrase; filenames stay as human-readable dates (`2026-06-04.md`)
- **File I/O:** entries written to `Documents\Journal\entries\YYYY\MM\` on create; read and decrypted on demand
- **Tauri commands:** `create_vault`, `unlock_vault`, `lock_vault`, `vault_status`, `create_entry`, `read_entry`, `update_entry`, `delete_entry`
- **Vault state machine:** `NoVault → Locked → Unlocked`; locking zeroes the key from memory (zeroize)

### Key decision: no key storage
The master password is never stored anywhere. The argon2 salt is stored (needed to re-derive the key), but the key itself exists only in RAM while the vault is unlocked. Get the password wrong and decryption fails — there is no backdoor and no recovery path by design.

---

## Phase 1 — Scaffold
**Commit:** `367f38d`

### What was built
- Tauri 2.0 project initialized with React 19 + TypeScript + Vite frontend
- Git repository, `.gitignore` configured
- App identity set: `productName: "Journal App"`, identifier `com.fourier18.journalapp`, version `0.1.0`, 1200×800 window
- Windows installer configured: NSIS, `installMode: currentUser` → installs to `%LOCALAPPDATA%\Programs\` (no admin rights required)
- Dev window verified running

---

## Metadata data-model decisions (for Phase 9 reference)

When Phase 9 (charts/visualizations) is built, the following are deliberately deferred — don't try to solve them earlier:

- **Scale drift** — if mood changes from 1–10 to 1–5 mid-way through a journal, the chart will look broken. Fix: let Phase 9 detect discontinuities and let the user annotate the scale change.
- **Unit consistency** — `sleep: 6.5` vs `sleep: "6h 30m"` stored by older entries before the "number or note?" choice was added. Fix: at chart time, attempt numeric parse, skip/count entries that can't be parsed, and surface a "N entries omitted" note.
- **Field name drift** — `mood` vs `my_mood` are separate keys. Fix: Phase 9 can offer a field-rename/merge tool.
- **Formal field schema** — a persisted schema with enforced types and ranges was considered and deferred. The current approach (type choice at field-creation, `MetadataValue::Number | Text` in storage) is the right foundation; more enforcement can be added when the charting requirements are clearer.

What *is* guaranteed from this point forward: any custom field the user creates as a "number" field will be stored as `MetadataValue::Number(f64)` from the start — so Phase 9 can reliably identify chartable fields by checking the value type.

---

## What's next

No phase numbering beyond 5 — the remaining work is an unordered backlog. Items will be tackled based on priority at the time, not a fixed sequence:

- Calendar view
- Charts / mood and habit visualizations
- Export to Markdown / PDF
- Attachments
- Custom theme editor
- Lock-screen refinements
- **Tag filtering of the entry list (re-do)** — the original sidebar tag filter was removed in the Phase 6 follow-up because it crowded the nav and was confused with the entry tag editor. If revived, do it cleanly: e.g. click a tag chip shown on an entry to filter the list to entries sharing it, rather than a standing panel of capsules. (`search_entries` already supports tag filtering on the Rust side, so this is a UI-only task.)

---

## Credits

- **Creative Director:** Joshua ([@Fourier18](https://github.com/Fourier18)) — concept, privacy and ownership requirements, storage-location and encryption-flavor decisions, phase planning, design direction, and sign-off
- **Engineer:** Claude (Anthropic) — Rust storage and crypto layer, Tauri command bridge, React UI, theming, test suite. Built across sessions.
