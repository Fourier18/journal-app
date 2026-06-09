# Journal App

A local-only, encrypted personal journal for Windows. Your entries live on your own machine — no cloud, no account, no telemetry.

## Principles

- **Local-only.** The app never makes a network call with your data.
- **Encrypted at rest.** Every entry is encrypted with [age](https://age-encryption.org/) using a key derived from your master password via argon2id. The password is never stored — lock the app and the key is wiped from memory.
- **Human-readable forever.** The decrypted format is plain Markdown + YAML frontmatter. The data outlives the app.
- **No lock-in.** Open format by design. An export step (coming) makes it one click to get a folder of normal Markdown files.

## How it works

Entries are written as Markdown with a YAML frontmatter block (tags, metadata, type, timestamps). Each file is encrypted whole before it touches disk — body, tags, and metadata all together. The encrypted `.md` files are the only on-disk copy of your content; there is no index file. On unlock, entries are decrypted into an in-memory index held in RAM alongside the key; both are dropped when you lock. Nothing decrypted is ever written back to disk.

Your master password is run through argon2id (64 MiB / 3 iterations) to derive the vault key. Get the password wrong and decryption fails — there is no recovery backdoor by design.

## Where your data lives

```
Documents\Journal\
├── entries\2026\06\2026-06-04.md       # encrypted entry, human-readable filename
├── attachments\                        # reserved
├── templates\                          # reserved
└── .journal\
    ├── config.toml
    ├── salt                            # argon2 salt (not the key)
    └── verify                          # tiny encrypted blob to check the password
```

The app installs per-user (no admin rights) to `%LOCALAPPDATA%\Programs\`.

## Features

- Encrypted vault with master password (create, unlock, lock)
- CodeMirror 6 editor with auto-save; light, dark, and sepia themes
- Daily and free-form entry types; six writing templates
- Editable titles; date shown as subtitle
- Tags — inline add/remove, dropdown picker, copy-tags-from-another-entry bundles
- Metadata panel — custom fields typed as number or text; click-to-edit
- Wikilinks — `[[` autocomplete links to any entry by title; renders as clickable label; broken links flagged
- Backlinks panel — see every entry that links to the current one
- Full-text search with scope toggles (body / title / tags / metadata), all-words or exact-phrase match, relevance or date sort, highlighted snippets, and jump-to-match in the editor

## Tech stack

- **Shell:** Tauri 2.0 (Rust + WebView2)
- **Frontend:** React 19, TypeScript, Vite, Zustand, CodeMirror 6, date-fns
- **Backend:** age (encryption), argon2 (key derivation), serde + serde_yaml, zeroize

## Credits

- **Creative Director:** Joshua ([@Fourier18](https://github.com/Fourier18))
- **Engineer:** Claude (Anthropic)
