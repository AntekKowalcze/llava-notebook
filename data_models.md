CREATE TABLE notes (
    local_id TEXT PRIMARY KEY,
    mongo_id TEXT,
    owner_id TEXT NOT NULL,
    
    name TEXT NOT NULL,
    title TEXT NOT NULL,
    content_path TEXT NOT NULL,
    
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,
    
    version INTEGER NOT NULL DEFAULT 1,
    cloud_version INTEGER NOT NULL DEFAULT 0,
    
    sync_state TEXT NOT NULL DEFAULT 'LocalOnly',
    is_deleted INTEGER NOT NULL DEFAULT 0,
    
    encrypted INTEGER NOT NULL DEFAULT 1,
    crypto_meta TEXT,
    
    UNIQUE(owner_id, name),
    CHECK(sync_state IN ('LocalOnly', 'PendingUpload', 'Synced', 'Conflict', 'Error'))
);

CREATE INDEX idx_notes_owner_updated ON notes(owner_id, updated_at DESC);
CREATE INDEX idx_notes_sync_state ON notes(sync_state);
CREATE INDEX idx_notes_mongo_id ON notes(mongo_id);

CREATE TABLE attachments (
    attachment_id TEXT PRIMARY KEY,
    note_local_id TEXT NOT NULL REFERENCES notes(local_id) ON DELETE CASCADE,
    
    filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    
    local_path TEXT,
    cloud_key TEXT,
    
    checksum_encrypted TEXT NOT NULL,
    
    encrypted INTEGER NOT NULL DEFAULT 1,
    crypto_meta TEXT,
    
    sync_state TEXT NOT NULL DEFAULT 'LocalOnly',
    
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    
    CHECK(sync_state IN ('LocalOnly', 'PendingUpload', 'Synced', 'Error'))
);

CREATE INDEX idx_attachments_note ON attachments(note_local_id);
CREATE INDEX idx_attachments_cloud_key ON attachments(cloud_key);
CREATE INDEX idx_attachments_sync_state ON attachments(sync_state);

CREATE TABLE tags (
    tag_id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL,
    name TEXT NOT NULL,
    color TEXT DEFAULT '#3B82F6',
    created_at INTEGER NOT NULL,
    UNIQUE(owner_id, name)
);

CREATE TABLE note_tags (
    note_local_id TEXT NOT NULL REFERENCES notes(local_id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(tag_id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (note_local_id, tag_id)
);

CREATE INDEX idx_note_tags_tag ON note_tags(tag_id);

CREATE TABLE sync_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);




MONGO

// ═══════════════════════════════════════════════════════
// NOTES COLLECTION
// ═══════════════════════════════════════════════════════
{
  "_id": ObjectId("..."),
  "local_id": "uuid-1234",
  "owner_id": "uuid-user1",
  "name": "project_notes",
  "title": "Rust Architecture Overview",
  
  "content": "<base64 of encrypted markdown>",
  
  "tags": ["rust", "architecture"],
  
  "attachments": [
    {
      "attachment_id": "uuid-5678",
      "cloud_key": "users/user1/notes/project_notes/img1.png",
      "filename": "img1.png",
      "mime_type": "image/png",
      "size_bytes": 245233,
      "checksum_encrypted": "sha256abc...",
      "encrypted": true
    }
  ],
  
  "created_at": 1730671200,
  "updated_at": 1730675200,
  "version": 3,
  
  "encrypted": true,
  "crypto_meta": {
    "algorithm": "ChaCha20-Poly1305",
    "salt": "abc123",
    "nonce": "xyz"
  },
  
  "is_deleted": false
}

// ═══════════════════════════════════════════════════════
// INDEXES
// ═══════════════════════════════════════════════════════
db.notes.createIndex({ "owner_id": 1, "local_id": 1 }, { unique: true });
db.notes.createIndex({ "owner_id": 1, "updated_at": -1 });
db.notes.createIndex({ "owner_id": 1, "is_deleted": 1 });
db.notes.createIndex({ "owner_id": 1, "tags": 1 });
