## Threat Model

- Protect note **content** from cloud/server compromise
- Server and MongoDB never see plaintext — **except when AI features are explicitly enabled per note** (see AI section)
- Note **titles and summaries** are plaintext in SQLite — accepted tradeoff (user controls what goes there)
- All crypto happens client-side in Rust, never on the server
- Blobs stored as raw `BLOB` in SQLite — no base64 encoding needed

---

## Crypto Architecture

### Primitives

- Encryption: **ChaCha20-Poly1305** everywhere
- Key derivation: **Argon2id**
- HTTP key exchange context: N/A — password based

### Key Hierarchy

```
Password
    └── Argon2id(password, local_kek_salt) → Local KEK
            └── ChaCha20Poly1305(Local KEK) → encrypts → Local Key (or Master Key)

Master Key (32 random bytes, generated once at online account creation)
    ├── wrapped by Local KEK → stored in local SQLite
    └── wrapped by Online KEK (Argon2id(online_password, kek_salt)) → stored in MongoDB
```

### Note Encryption

- Notes encrypted with **Local Key** (no online account) or **Master Key** (online linked)
- Each note has its own `nonce` (12 bytes, random, stored with ciphertext)
- `key_type` field on each note tells the app which key to use for decryption

### Image/Attachment Encryption

- Same key as notes (local or master depending on note's key_type)
- Each attachment has its own nonce
- Encrypted before leaving the device, S3 never sees plaintext

---

## Local Storage

### SQLite — Notes

```sql
notes (
    local_id, mongo_id, owner_id,
    name,                   -- filename on disk (plaintext, user controls)
    title,                  -- first line preview (plaintext, user controls)
    summary,                -- short preview (plaintext, user controls)
    content_path,           -- path to encrypted content file
    sync_intent,            -- 'Local' | 'Online'  (what user WANTS)
    sync_state,             -- 'LocalOnly' | 'PendingUpload' | 'Synced' | 'Conflict' | 'Error' | 'PendingDeleted'
    key_type,               -- 'local' | 'master'
    version, cloud_version,
    vector_clock,
    created_at, updated_at, deleted_at,
    is_deleted, encrypted,
    crypto_meta,            -- nonce, salt as JSON
    ai_summary BLOB,        -- encrypted AI summary (same key as note, key_type applies)
    ai_summary_nonce BLOB,  -- fresh nonce for AI summary
    ai_summary_updated_at INTEGER  -- to detect stale summaries after note edits
)
```

### SQLite — Attachments

```sql
attachments (
    attachment_id, note_local_id,
    filename, mime_type, size_bytes,
    local_path,     -- relative path e.g. "images/uuid.jpg"
    cloud_key,      -- S3 key
    checksum_encrypted,
    sync_state,     -- 'LocalOnly' | 'PendingUpload' | 'Synced' | 'Error' | 'PendingDeleted'
    crypto_meta,    -- nonce
    created_at, updated_at
)
```

### SQLite — Local User

```rust
LocalUser {
    user_id: Uuid,
    mongo_id: Option<String>,           // None until online linked
    username: String,
    password_hash: String,              // argon2id

    local_key_enc: Vec<u8>,             // local key wrapped by local KEK
    local_key_nonce: Vec<u8>,
    local_kek_salt: String,             // SaltString (argon2 crate)

    master_key_enc: Option<Vec<u8>>,    // None until online linked
    master_key_nonce: Option<Vec<u8>>,
    master_kek_salt: Option<String>,    // separate salt — critical, never reuse

    is_online_linked: bool,
    online_account_email: Option<String>,
    device_id: Uuid,
    created_at: i64,
    last_login: i64,
    password_errors: i64,
    ending_block_timestamp: i64,
}
```

### Note Content Files

- Stored at `content_path` (encrypted with ChaCha20-Poly1305)
- Images stored at relative paths e.g. `images/uuid.jpg` (also encrypted)
- Markdown references images by relative path: `![](images/uuid.jpg)`
- Paths always use forward slashes regardless of OS

### SQLite Migrations

- Track schema version with `PRAGMA user_version`
- Read with: `SELECT * FROM pragma_user_version()`
- Write with: `conn.pragma_update(None, "user_version", N)?`
- Each migration wrapped in a transaction, version set before commit
- Never edit old migrations, only add new ones

---

## Cloud Storage

### MongoDB Collections

**users**

```json
{
  "_id": "uuid",
  "email": "string",
  "email_verified": "bool",
  "password_hash": "argon2id hash",
  "master_key_enc": "base64",
  "master_key_nonce": "base64",
  "kek_salt": "base64",
  "argon2_params": { "m_cost": 65536, "t_cost": 3, "p_cost": 4 },
  "storage_used_bytes": 0,
  "quota_bytes": 1073741824,
  "failed_attempts": 0,
  "lockout_until": null,
  "created_at": "timestamp",
  "last_login": "timestamp"
}
```

**devices**

```json
{
  "_id": "uuid",
  "user_id": "uuid",
  "device_name": "string",
  "last_seen": "timestamp",
  "created_at": "timestamp"
}
```

**notes** (encrypted blobs + metadata only)

```json
{
  "_id": "note_id",
  "user_id": "uuid",
  "nonce": "<binary>",
  "ciphertext": "<binary>",
  "ai_summary": "<binary>",
  "ai_summary_nonce": "<binary>",
  "ai_summary_updated_at": "timestamp",
  "created_at": "timestamp",
  "updated_at": "timestamp",
  "vector_clock": { "device_a": 5 },
  "attachments": [
    {
      "id": "uuid",
      "s3_key": "users/uid/uuid.enc",
      "nonce": "<binary>",
      "relative_path": "images/uuid.jpg",
      "mime_type": "image/jpeg",
      "size_bytes": 204800
    }
  ]
}
```

**refresh_tokens**

```json
{
  "_id": "uuid",
  "user_id": "uuid",
  "token_hash": "sha256 of token",
  "device_id": "uuid",
  "expires_at": "timestamp",
  "created_at": "timestamp"
}
```

### S3

- Images and attachments only (not notes text)
- Files stored as raw encrypted blobs (`.enc`)
- Key format: `users/{user_id}/{attachment_id}.enc`
- Client never has S3 credentials — server issues **presigned URLs** (15 min TTL)
- Client uploads directly to S3 via presigned URL (server not in data path)

---

## Sync Architecture

### What syncs

- Notes with `sync_intent = 'Online'` and `key_type = 'master'`
- Local-only notes (`sync_intent = 'Local'`) stay on device forever

### Sync flow

```
client pulls note versions from server
    ↓
compare vector_clock / updated_at with local SQLite
    ↓
local newer  → push encrypted blob to server
server newer → pull blob, save locally
both changed → conflict (flag as 'Conflict', user resolves)
```

### Attachment sync

```
on image paste/drop:
    generate uuid filename
    save encrypted to local "images/uuid.jpg"
    insert ![](images/uuid.jpg) into markdown
    queue for S3 upload

on sync to new device:
    pull note from MongoDB (gets attachment metadata + s3_key)
    download .enc from S3
    decrypt → save to same relative path
    note renders correctly
```

### Deduplication

- SHA-256 of encrypted ciphertext before upload
- Ask server "do you have this hash?" before uploading
- Skip upload if already exists, just update MongoDB reference

---

## Linking a New Device Flow

**Device B already has local notes:**

```
1. prompt: enter current local password → decrypt existing notes
2. prompt: enter online password → fetch + decrypt master key from MongoDB
3. re-encrypt all 'Online'-intent notes with master key (one-time migration)
4. wrap master key with local password → store in SQLite
5. device is linked, offline access works forever after
```

**Device B has no local notes:**

```
1. enter online password → fetch + decrypt master key
2. wrap master key with local password → store in SQLite
3. done (no migration needed)
```

---

## Server (Go + Fiber)

### What it is

A thin authenticated gateway — not a business logic server. Clients talk to it; it verifies identity and proxies to MongoDB/S3.

### What it does

- Verify JWT on every request
- Enforce "you can only touch your own data"
- Enforce storage quotas before issuing presigned URLs
- Generate S3 presigned URLs
- Handle lockout after failed login attempts
- Track storage_used_bytes

### What it does NOT do

- Crypto (never)
- See plaintext content (never)
- Data transformation

### Endpoints

```
POST /auth/register
POST /auth/login
POST /auth/refresh
POST /auth/logout
POST /auth/logout-all
GET  /devices
DELETE /devices/:id
GET  /notes
POST /notes
PUT  /notes/:id
DELETE /notes/:id
POST /attachments/upload-url   ← returns presigned S3 PUT URL
GET  /storage/usage
GET  /health
```

### Auth — JWT

- **Access token**: short-lived (15 min), in memory only, never persisted
- **Refresh token**: long-lived (7-30 days), stored in OS keyring
- Refresh token stored as SHA-256 hash in MongoDB (raw token never in DB)
- Refresh token rotation: old invalidated on use, new issued
- If old refresh token used again → all tokens invalidated (theft detected)
- JWT payload: `{ sub: user_id, device_id, exp, iat }`
- Two secrets: `ACCESS_TOKEN_SECRET` and `REFRESH_TOKEN_SECRET`
- DB lookup only on login and token refresh, not every request

### Deployment

- **mikr.us VPS** (cheap, good for development and early production)
- Go server listens on plain HTTP `:8080`
- **Caddy** as reverse proxy handles TLS automatically (Let's Encrypt, auto-renews)
- Need a domain pointed at VPS IP — Caddy handles the rest
- MongoDB and S3 stay as external managed services (Atlas free tier + Backblaze B2)

```
internet (HTTPS :443) → Caddy → Go :8080 → MongoDB Atlas
                                           → S3 / Backblaze B2
```

---

## AI Features (Ollama)

### Opt-in model

- AI features are **explicitly enabled per-user in settings** — off by default
- User is informed the server will see note content temporarily during processing
- Only available for online-linked accounts (requires master key infrastructure)

### Flow

```
user triggers "summarize note"
    ↓
Rust decrypts note locally → plaintext in RAM
    ↓
client sends plaintext to server over HTTPS
    ↓
server feeds to Ollama → gets summary plaintext
server returns summary plaintext to client
server holds nothing — RAM only, zeroized after
    ↓
client encrypts summary with same master key, fresh nonce
stores ai_summary BLOB + ai_summary_nonce in SQLite
syncs to MongoDB with note on next sync
```

### Key design

- No new keys — AI summary uses the **same key_type as the note** (always master for online notes)
- Only extra thing needed is a fresh nonce per summary (nonce is not secret)
- `ai_summary_updated_at` vs `updated_at` tells UI when summary is stale after edits

### Server addition

```
POST /ai/summarize   ← receives plaintext, returns plaintext summary
```

### What does NOT happen

- Server never encrypts anything
- Server never stores plaintext
- No per-note AI keys — same key machinery as everything else

---

## Request Architecture (Rust vs Vue)

**Rust handles** (sensitive operations):

- All auth requests (login, refresh, logout)
- Note sync (reads SQLite before push, writes SQLite after pull)
- Anything touching master key, local key, or tokens

**Vue handles** (display only):

- Direct S3 uploads via presigned URL (not sensitive, just file bytes)
- Storage usage display
- UI state

---

## File Upload Safety

```
client-side: check size < 10MB, check mime type is image/*
server-side: enforce same limits before issuing presigned URL
             check user quota before issuing presigned URL
S3-side:     presigned URL has ContentLengthRange condition (max 10MB)
```

Three independent layers — all must be bypassed to abuse storage.

---

## UX Decisions

- Password (not PIN) for local auth — security is only as strong as the weakest point
- Per-note sync toggle visible in note editor (cloud/lock icon)
- Global default sync setting (on/off)
- "Convert all notes to online" button — requires online account linked
- Notes marked `sync_intent = 'Online'` before account exists are queued — migrated automatically at link time
- Local-only notes never touched during device linking
- Forgotten online password = unrecoverable synced data (consider showing recovery key at registration)
- Password reset flow is cryptographically complex — plan carefully (must re-wrap master key)
- AI features opt-in in settings with explicit privacy warning — server sees plaintext during processing
- "Summary outdated" indicator when `ai_summary_updated_at` < `updated_at`

---

## Crates to Use

- `chacha20poly1305` — RustCrypto, encryption
- `argon2` — RustCrypto, key derivation (SaltString type)
- `reqwest` — HTTP client (reuse Client, store in AppState)
- `keyring` — refresh token storage
- `rusqlite` — SQLite
- `uuid` — IDs
- `serde` / `serde_json` — serialization
- `zeroize` — wipe key material from memory
