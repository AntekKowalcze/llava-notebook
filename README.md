# 🧱 Llava Architecture

## 🎯 Project Goals

* Offline-first note-taking app with optional sync via MongoDB and S3.
* Local storage in Markdown files + metadata in SQLite.
* Full end-to-end encryption (E2E) of all user data.
* Each user has a separate profile and local database.
* Desktop application built on **Tauri (Rust + Vue)**.

***

## ⚙️ Tech Stack

* **Frontend:** Vue + TypeScript
* **Backend:** Rust (tokio, serde, rusqlite, mongodb, aws-sdk-s3)
* **Local DB:** SQLite (with `bundled` feature)
* **Cloud DB:** MongoDB Atlas
* **File/attachment storage:** AWS S3 (encrypted)
* **Encryption:** ChaCha20-Poly1305 + Argon2id (E2E)
* **Markdown parser:** pulldown_cmark / comrak

***

## 🗂️ Local Data Structure

```
~/llava/
  users/
    <user_uuid>/
      db.sqlite              # Metadata and history
      notes/<uuid>.md        # Note content (Markdown)
      assets/<uuid>/...      # Images and attachments
      logs/app.log
```

***

## 📄 Data Model
`todo`

### ☁️ MongoDB (cloud database)
In data models file.

***

## 🔐 Security

* End-to-end encryption (E2E): notes and files encrypted locally.
* Master key derived from user's password (Argon2id).
* Encrypted master key stored locally in SQLite.
* S3 and MongoDB only ever see encrypted data.
* Lost key = no decryption possible (backup required).
* Lost password → recovery codes, each one wraps the notes key (KEK).

***

## 🔄 Synchronization

* **Offline-first** — everything works locally, even without internet.
* **SyncManager** every 30 seconds or on `Ctrl+S`:
  1. Pushes local changes to Mongo/S3 (upsert per `local_id`).
  2. Pulls remote changes (`updated_at` > `last_sync`).
  3. Resolves conflicts (last-writer-wins + history snapshots).
  4. Emits events: `sync:started`, `sync:progress`, `sync:completed`.
* `sync_ops` queue in SQLite stores all pending changes.

***

## 🧹 Attachment Management

* Every attachment has `checksum_encrypted` and `sync_state`.
* Upload to S3 happens after encryption (streaming).
* Deterministic S3 keys (`user_id/local_id/attachment_id`).
* `AttachmentCleaner` removes orphaned files locally and in S3.

***

## 🧠 Additional Mechanisms

* **History snapshots:** every note edit saves a local version (`history/<note_id>/v{n}.md`).
* **Logger:** logs CRUD, sync, crypto operations and errors.
* **Tauri Event System:** Rust ↔ frontend communication (`sync:started`, `note:created`, `error:network`).
* **Offline mode:** user can manually disable synchronization.

***

## 🧰 Supporting Technologies

* `rusqlite` – local database (with `bundled` feature)
* `mongodb` – MongoDB client
* `aws-sdk-s3` – upload and download of encrypted files
* `tokio` – async runtime
* `uuid`, `chrono`, `serde`, `argon2`, `chacha20poly1305`

***

## 🚀 Development Plan (iterative)

| # | Milestone |
|---|---|
| 0 | Local registration/login + frontend |
| 1 | Local CRUD (Markdown + SQLite) |
| 2 | Basic note sync with MongoDB |
| 3 | Local AttachmentManager + S3 upload |
| 4 | E2E encryption of notes and attachments |
| 5 | Logger + version history + conflict manager |
| 6 | Tauri event integration + UI notifications |
| 7 | AI summary (local, self-hosted, private) |

***

## 📊 Core Design Principles

* Every component has a **single responsibility** (SRP).
* Data **always local first**, sync later.
* Synchronization is **idempotent** (no duplicates).
* No intermediate server — the client encrypts and sends data directly.
* Code and logic prepared for **multi-device sync**.

***

## 🏗️ Architecture Diagram

```
        ┌────────────────────────────┐
        │        🖥️  Frontend        │
        │       Vue (Tauri UI)       │
        │                            │
        │  • Markdown editor         │
        │  • Live preview            │
        │  • Note list / tags        │
        │  • Notifications & events  │
        └──────────────┬─────────────┘
                       │
             Tauri Commands + Events
                       │
                       ▼
┌───────────────────────────────────────────────────┐
│             ⚙️  Rust Backend (Tauri)              │
│───────────────────────────────────────────────────│
│  AppState (shared state)                          │
│  ├── StorageService    CRUD on files and SQLite   │
│  ├── SyncManager       Sync with Mongo/S3         │
│  ├── CryptoService     Encryption / decryption    │
│  ├── AttachmentManager Attachment management      │
│  ├── AuthService       Login/register/masterkey   │
│  ├── Cleaner           Remove orphaned files      │
│  └── Logger            Audit and diagnostics      │
└──────────────────────┬────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────┐
│        💾  Local Storage (Offline)           │
│──────────────────────────────────────────────│
│  ~/.llava/users/<uuid>/                      │
│  ├── notes/*.md       Markdown notes         │
│  ├── assets/*         Images and attachments │
│  ├── db.sqlite        Metadata and history   │
│  ├── tmp/             Files being written    │
│  └── delete_tmp/      Soft-deleted notes     │
│                                              │
│  → Offline-first writes                      │
│  → E2E encryption                            │
└───────────────────┬──────────────────────────┘
                    │
              Sync (every 30s / Ctrl+S)
                    │
                    ▼
┌──────────────────────────────────────────────┐
│             ☁️  Cloud Backend                │
│──────────────────────────────────────────────│
│  MongoDB Atlas  → Note metadata (JSON)       │
│  S3 Storage     → Encrypted files & images   │
│                                              │
│          🔒 Everything encrypted             │
│           → ChaCha20-Poly1305                │
│           → Key derived with Argon2id        │
│                                              │
│      🧹 Cleaner maintains consistency        │
│         (removes orphaned files)             │
└───────────────────┬──────────────────────────┘
                    │
                    ▼
  ┌────────────────────────────────────┐
  │  🔄  Bidirectional Synchronization │
  │────────────────────────────────────│
  │ 1. Local changes  → Mongo/S3       │
  │ 2. Remote changes → local cache    │
  │ 3. Conflicts      → last-write-wins│
  │ 4. History and snapshots           │
  │ 5. Events to UI (progress, error)  │
  └────────────────────────────────────┘
```

***

## 🤖 AI

* Note summaries generated by a local AI hosted on your own server — private by design, with more AI features planned. backend on serwer written by me

+ Methapone setting lookup (writen from scratch but on level that is needed)
+ fuzzy search written from scratch (same as above only on level needed to understand and learn)