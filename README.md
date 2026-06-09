# Journal App

A local, encrypted journal for Windows. Your entries live on your own machine — no cloud, no account, no telemetry.

The guiding idea: a journal you fully own. Files are readable by ordinary tools the day you stop using this app, but unreadable to anyone without your password while you're using it. Durability and privacy first; lock-in never.

## Principles

- **Local-only.** The app never makes a network call with your data.
- **Encrypted at rest.** Every entry is encrypted with [age](https://age-encryption.org/); the key is derived from your master password with argon2id and never stored — lock the app and the key is wiped from memory.
- **Human-readable forever.** The decrypted format is plain Markdown + YAML frontmatter. The data outlives the app.
- **No vendor lock-in.** Open format by design; entries can be read elsewhere or exported.

## Features

- **Vault** — create once with a master password; unlock/lock on each session
- **Editor** — CodeMirror 6 with Markdown syntax hints, auto-save, and light / dark / sepia themes
- **Entry types** — Daily and Free-form, with six writing templates
- **Titles, tags, and metadata** — editable title on every entry; inline tag picker; metadata panel with mood, sleep, custom number and text fields
- **Wikilinks** — type `[[` to link any entry by title; renders as a clickable label; broken links shown with strikethrough
- **Backlinks** — each entry shows which other entries link to it
- **Search** — full-text search with scope toggles (body / title / tags / metadata), all-words or exact-phrase matching, relevance or date sorting, and highlighted snippets in results

## How it works

Entries are stored as age-encrypted `.md` files under `Documents\Journal\entries\`. On unlock, all files are decrypted into an in-memory index (held in RAM alongside the key) to power listing, search, and linking. That index is dropped on lock. Nothing decrypted is ever written to disk.

```
Documents\Journal\
├── entries\2026\06\2026-06-04.md   # encrypted entry, human-readable filename
├── attachments\                    # reserved
├── templates\                      # reserved
└── .journal\
    ├── salt                        # argon2 salt (not the key)
    └── verify                      # tiny encrypted blob to confirm the password
```

## Tech stack

- **Shell:** Tauri 2.0 (Rust + WebView2)
- **Frontend:** React 19, TypeScript, Vite, Zustand, CodeMirror 6, date-fns
- **Backend:** age, argon2, serde/serde_yaml, zeroize

## Credits

- **Creative Director:** Joshua ([@Fourier18](https://github.com/Fourier18))
- **Engineer:** Claude (Anthropic)
