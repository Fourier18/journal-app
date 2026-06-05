# Journal App

The guiding idea: a journal you fully own. The files are readable by ordinary tools the day you decide to stop using this app, but unreadable to anyone who doesn't have your password while you're using it. Durability and privacy first; lock-in never.

## Principles

- **Local-only.** No server, no sync, no analytics. The app never makes a network call with your data.
- **Encrypted at rest.** Every entry file is encrypted with [age](https://age-encryption.org/); the key is derived from your master password with argon2id. The password is never stored — lock the app and the key is wiped from memory.
- **Human-readable forever.** Filenames stay as plain dates (`2026-06-04.md`). The decrypted format is Markdown + YAML frontmatter, so the data outlives the app. If this project disappeared tomorrow, an export gives you a folder of normal Markdown.
- **No vendor lock-in.** Built on an open format on purpose, so the entries can be read elsewhere later (a future export step makes that one click).

## How it works

- Entries are written as Markdown with a YAML frontmatter block (tags, metadata, type, timestamps).
- Each file is encrypted whole — body, tags, and metadata all — before it touches disk.
- The encrypted `.md` files are the **only** on-disk copy of your content. There is no content index file. When you unlock, the app decrypts your entries into an in-memory index (held only in RAM, alongside the key) to power listing, search, and tag filtering; that index is dropped when you lock. Nothing decrypted is ever written back to disk.
- Your master password is run through argon2id (64 MiB / 3 iterations) to derive the vault key, which is used as the age passphrase. Get the password wrong and decryption simply fails — there's no recovery backdoor by design.

## Where your data lives

```
Documents\Journal\
├── entries\2026\06\2026-06-04.md       # encrypted entry, human-readable filename
├── attachments\                        # (reserved for future use)
├── templates\                          # (reserved for future use)
└── .journal\
    ├── config.toml
    ├── salt                            # argon2 salt (not the key)
    └── verify                          # tiny encrypted blob to check the password
```

No content index is stored on disk — the encrypted entry files are the only copy.

The app installs per-user (no admin rights needed) to `%LOCALAPPDATA%\Programs\`.

## Tech stack

- **Shell:** Tauri 2.0 (Rust backend + WebView2 frontend)
- **Frontend:** React 19, TypeScript, Vite, Zustand, CodeMirror 6, date-fns
- **Backend (Rust):** age (encryption), argon2 (key derivation), serde + serde_yaml, zeroize (memory wiping on lock). The entry index is in-memory only — no database file.

## Status

**Work in progress.** What's built and verified today:

- ✅ Encrypted vault: create / unlock / lock with a master password
- ✅ Entry storage — age-encrypted Markdown, confirmed encrypted on disk
- ✅ Editor (CodeMirror) with auto-save, light / dark / sepia themes
- ✅ Lock screen (first-run setup + returning unlock)
- ✅ Six writing templates, daily and free-form entry types
- ✅ Tags, a metadata panel (mood / sleep / custom fields), inline delete
- ✅ Full-text search and tag filtering — running over the in-memory index, no plaintext on disk
- ✅ 18 passing Rust tests (incl. a guard that fails if any plaintext index ever reappears); TypeScript compiles clean

Not yet built (an unordered backlog, nothing committed to a fixed order): calendar view, charts, export to Markdown/PDF, attachments, a custom theme editor, and lock-screen refinements.

> Earlier builds kept a plain-SQLite index that stored decrypted entry text on disk — a leak that undercut the encryption. That's been removed: the index is now in-memory only, and old index files are purged automatically on unlock. An optional encrypted on-disk index (SQLCipher) remains a possible future step if search ever needs to scale.

## Credits

- **Creative Director:** Joshua ([@Fourier18](https://github.com/Fourier18)) — concept, privacy and ownership requirements, storage-location and encryption-flavor decisions, phase planning, design direction, and sign-off
- **Engineer:** Claude (Anthropic) — Rust storage and crypto layer, Tauri command bridge, React UI, theming, test suite. Built across sessions, primarily on Sonnet.
