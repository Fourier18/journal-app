# Journal App

A private, local-only journal for Windows. Your entries live as plain Markdown files on your own disk, encrypted at rest — no cloud, no account, no telemetry. Nothing leaves the machine.

The guiding idea: a journal you fully own. The files are readable by ordinary tools the day you decide to stop using this app, but unreadable to anyone who doesn't have your password while you're using it. Durability and privacy first; lock-in never.

## Principles

- **Local-only.** No server, no sync, no analytics. The app never makes a network call with your data.
- **Encrypted at rest.** Every entry file is encrypted with [age](https://age-encryption.org/); the key is derived from your master password with argon2id. The password is never stored — lock the app and the key is wiped from memory.
- **Human-readable forever.** Filenames stay as plain dates (`2026-06-04.md`). The decrypted format is Markdown + YAML frontmatter, so the data outlives the app. If this project disappeared tomorrow, an export gives you a folder of normal Markdown.
- **No vendor lock-in.** Built on an open format on purpose, so the entries can be read elsewhere later (a future export step makes that one click).

## How it works

- Entries are written as Markdown with a YAML frontmatter block (tags, metadata, type, timestamps).
- Each file is encrypted whole — body, tags, and metadata all — before it touches disk.
- A local SQLite index (in `.journal/`) tracks entries for fast listing and future search; it holds no plaintext you'd mind, and is itself slated to move under encryption later.
- Your master password is run through argon2id (64 MiB / 3 iterations) to derive the vault key, which is used as the age passphrase. Get the password wrong and decryption simply fails — there's no recovery backdoor by design.

## Where your data lives

```
Documents\Journal\
├── entries\2026\06\2026-06-04.md       # encrypted entry, human-readable filename
├── attachments\                        # (reserved for future use)
├── templates\                          # (reserved for future use)
└── .journal\
    ├── index.db                        # local SQLite index
    ├── config.toml
    └── salt                            # argon2 salt (not the key)
```

The app installs per-user (no admin rights needed) to `%LOCALAPPDATA%\Programs\`.

## Tech stack

- **Shell:** Tauri 2.0 (Rust backend + WebView2 frontend)
- **Frontend:** React 19, TypeScript, Vite, Zustand, CodeMirror 6, date-fns
- **Backend (Rust):** age (encryption), argon2 (key derivation), rusqlite (index), serde + serde_yaml, zeroize (memory wiping on lock)

## Status

**Work in progress.** What's built and verified today:

- ✅ Encrypted vault: create / unlock / lock with a master password
- ✅ Entry storage — age-encrypted Markdown, confirmed encrypted on disk
- ✅ Editor (CodeMirror) with auto-save, light / dark / sepia themes
- ✅ Lock screen (first-run setup + returning unlock)
- ✅ Six writing templates, daily and free-form entry types
- ✅ Tags, a metadata panel (mood / sleep / custom fields), inline delete
- ✅ 13 passing Rust tests; TypeScript compiles clean

Not yet built (an unordered backlog, nothing committed to a fixed order): full-text search, tag filtering, calendar view, charts, export to Markdown/PDF, attachments, a custom theme editor, and lock-screen refinements.

> Encryption note: the index database is currently plain SQLite. Moving it under SQLCipher is a known future hardening step, deferred only because its Windows build toolchain is heavy. Entry *contents* are already fully encrypted today.

## Credits

- **Creative Director:** Joshua ([@Fourier18](https://github.com/Fourier18)) — concept, privacy and ownership requirements, storage-location and encryption-flavor decisions, phase planning, design direction, and sign-off
- **Engineer:** Claude (Anthropic) — Rust storage and crypto layer, Tauri command bridge, React UI, theming, test suite. Built across sessions, primarily on Sonnet.
