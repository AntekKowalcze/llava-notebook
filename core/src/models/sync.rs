use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{crypto::NoteCryptoMetadata, services::attachment::AttachmentCryptoMetadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckSyncRequest {
    #[serde(rename = "to_check")]
    pub notes: Vec<CheckNoteSyncStatus>,
    pub full_sync: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckNoteSyncStatus {
    pub local_id: String,
    pub cloud_id: Option<String>,
    pub hard_deleted: bool,
    pub cloud_version: Option<i64>,
    pub attachments: Option<Vec<AttachmentSyncCheck>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentSyncCheck {
    pub attachment_id: Uuid,
    pub checksum_encrypted: String,
    pub size_bytes: i64,
    pub is_encrypted: bool,
    pub hard_deleted: bool,
    pub file_name: String,
    pub mime_type: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub crypto_metadata: Option<AttachmentCryptoMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckSyncResponse {
    pub to_upload: Vec<String>,
    pub notes_to_download: Vec<DownloadNote>,
    pub notes_synced: Vec<String>,
    pub attachments_to_upload: Vec<UploadAttachment>,
    pub attachments_to_download: Vec<DownloadAttachment>,
    pub attachments_synced: Vec<String>,

    #[serde(default)]
    pub attachments_to_hard_delete: Vec<String>,

    pub notes_to_hard_delete: Vec<String>,

    #[serde(default)]
    pub notes_failed: Vec<String>,

    #[serde(default)]
    pub quota_exceeded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadAttachment {
    pub attachment_id: String,
    pub upload_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadNote {
    pub cloud_id: String,

    // Server-side only. This should normally never be sent by the server.
    #[serde(skip_serializing, skip_deserializing)]
    pub owner_id: String,

    pub cloud_version: i64,

    pub title: String,
    pub summary: String,
    pub content: String,

    pub created_at: i64,
    pub updated_at: i64,

    pub is_deleted: bool,
    pub deleted_at: Option<i64>,

    pub hard_deleted: bool,
    pub is_encrypted: bool,

    // Server-side only.
    #[serde(skip_serializing, skip_deserializing)]
    pub deleted_attachments: Option<Vec<String>>,

    pub crypto_meta: Option<NoteCryptoMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadAttachment {
    pub attachment_id: Uuid,

    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,

    pub cloud_key: String,
    pub checksum_encrypted: String,

    pub is_encrypted: bool,
    pub note_cloud_id: String,
    pub crypto_meta: Option<AttachmentCryptoMetadata>,

    pub created_at: i64,
    pub updated_at: i64,

    pub download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteAttachment {
    pub attachment_id: Uuid,
    pub cloud_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoteForUpload {
    pub local_id: String,
    pub mongo_id: Option<String>,
    pub owner_id: String,
    pub title: String,
    pub summary: String,
    pub content_path: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub version: i64,
    pub cloud_version: Option<i64>,
    pub sync_state: String,
    pub is_deleted: bool,
    pub encrypted: bool,
    pub hard_deleted: bool,
    pub crypto_meta: Option<String>,
    pub content: String,
}

#[derive(Debug)]
pub struct AttachmentForUpload {
    pub attachment_id: String,
    pub note_local_id: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub local_path: Option<String>,
    pub cloud_key: Option<String>,
    pub checksum_encrypted: String,
    pub encrypted: bool,
    pub crypto_meta: Option<String>,
    pub sync_state: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub note_cloud_id: String,
    pub content: Vec<u8>,
}
