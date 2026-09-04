package models

import (
	"time"

	"github.com/google/uuid"
	"go.mongodb.org/mongo-driver/v2/bson"
)

// **Client sends metadata of all notes that participate in sync.**
// **LocalOnly and WaitingForTombstone notes are not included.**

type CheckSyncRequest struct {
	Notes    []CheckNoteSyncStatus `json:"to_check" validate:"dive"`
	FullSync bool                  `json:"full_sync"`
}

type CheckNoteSyncStatus struct {
	LocalID     string         `bson:"local_id" json:"local_id" validate:"required"`
	SyncState   string         `json:"sync_state" validate:"required"`
	CloudID     *bson.ObjectID `bson:"_id,omitempty" json:"cloud_id,omitempty"`
	HardDeleted bool           `bson:"hard_deleted" json:"hard_deleted"`

	// **nil = note has never been uploaded**
	CloudVersion *int64 `bson:"cloud_version,omitempty" json:"cloud_version,omitempty"`

	Attachments []AttachmentSyncCheck `json:"attachments,omitempty" validate:"dive"`
}

type AttachmentSyncCheck struct {
	AttachmentID      uuid.UUID            `json:"attachment_id" validate:"required"`
	ChecksumEncrypted string               `json:"checksum_encrypted" validate:"required"`
	SizeBytes         int64                `json:"size_bytes"`
	IsEncrypted       bool                 `json:"is_encrypted"`
	FileName          string               `json:"file_name" validate:"required"`
	HardDeleted       bool                 `json:"hard_deleted"`
	MimeType          string               `json:"mime_type" validate:"required"`
	CreatedAt         int64                `bson:"created_at" json:"created_at"`
	UpdatedAt         int64                `bson:"updated_at" json:"updated_at"`
	CryptoMetadata    AttachmentCryptoMeta `json:"crypto_metadata,omitempty"`
}

type CheckSyncResponse struct {
	// **Notes that the client needs download**
	ToUpload []string `json:"to_upload"`

	// **Notes that the client needs to upload.**
	NotesToDownload []DownloadNote `json:"notes_to_download"`

	NotesSynced []string `json:"notes_synced"`

	// **Attachments that the client needs to download.**
	AttachmentsToUpload []UploadAttachment `json:"attachments_to_upload"`

	AttachmentsToDownload []DownloadAttachment `json:"attachments_to_download"`

	AttachmentsSynced []string `json:"attachments_synced"`

	AttachmentsToHardDelete []string `json:"attachments_to_hard_delete"` // **NEW**

	// **Notes that should be permanently deleted from the client.**
	NotesToHardDelete []string `json:"notes_to_hard_delete"`

	NotesFailed   []string `json:"notes_failed"`
	QuotaExceeded bool     `json:"quota_exceeded"`
}

type UploadAttachment struct {
	AttachmentId string `json:"attachment_id"`
	UploadUrl    string `json:"upload_url"`
}

type DownloadNote struct {
	CloudID      bson.ObjectID `bson:"_id" json:"cloud_id" validate:"required"`
	OwnerID      string        `bson:"owner_id" json:"-"`
	CloudVersion int64         `bson:"cloud_version" json:"cloud_version" validate:"required"`

	Title   string `bson:"title" json:"title"`
	Summary string `bson:"summary" json:"summary"`
	Content string `bson:"content" json:"content"`

	CreatedAt64 int64 `bson:"created_at" json:"created_at"`
	UpdatedAt64 int64 `bson:"updated_at" json:"updated_at"`

	IsDeleted bool   `bson:"is_deleted" json:"is_deleted"`
	DeletedAt *int64 `bson:"deleted_at,omitempty" json:"deleted_at,omitempty"`

	HardDeleted bool `bson:"hard_deleted" json:"hard_deleted"` //**on rust side when sync State = Waiting for tombstone**

	IsEncrypted bool `bson:"is_encrypted" json:"is_encrypted"`

	DeletedAttachments []string        `bson:"deleted_attachments,omitempty" json:"-"`
	CryptoMeta         *NoteCryptoMeta `bson:"crypto_meta,omitempty" json:"crypto_meta,omitempty"`
}

type DownloadAttachment struct {
	AttachmentID uuid.UUID `json:"attachment_id" validate:"required"`

	FileName  string `json:"file_name"`
	MimeType  string `json:"mime_type"`
	SizeBytes int64  `json:"size_bytes"`

	CloudKey          string `json:"cloud_key" validate:"required"` //**metadata returns to string**
	CloudNoteId       string `json:"note_cloud_id" validate:"required"`
	ChecksumEncrypted string `json:"checksum_encrypted" validate:"required"`

	IsEncrypted bool                  `json:"is_encrypted"`
	CryptoMeta  *AttachmentCryptoMeta `json:"crypto_meta,omitempty"`
	CreatedAt   int64                 `json:"created_at"`
	UpdatedAt   int64                 `json:"updated_at"`
	DownloadUrl string                `json:"download_url"`
}

type DeleteAttachment struct {
	AttachmentID uuid.UUID `json:"attachment_id" validate:"required"`
	CloudKey     string    `json:"cloud_key" validate:"required"`
}

type NoteCryptoMeta struct {
	TitleNonce   string `bson:"title_nonce" json:"title_nonce"`
	SummaryNonce string `bson:"summary_nonce" json:"summary_nonce"`
	ContentNonce string `bson:"content_nonce" json:"content_nonce"`
}

type AttachmentCryptoMeta struct {
	AttachmentNonce string `bson:"attachment_nonce" json:"attachment_nonce"`
}

type NoteUploadRequest struct {
	LocalID string  `json:"local_id" validate:"required"`
	MongoID *string `json:"mongo_id,omitempty"`

	Title   string `json:"title"`
	Summary string `json:"summary"`

	CreatedAt int64  `json:"created_at"`
	UpdatedAt int64  `json:"updated_at"`
	DeletedAt *int64 `json:"deleted_at,omitempty"`

	Version      int64  `json:"version"`
	CloudVersion *int64 `json:"cloud_version,omitempty"`

	SyncState   string `json:"sync_state"`
	IsDeleted   bool   `json:"is_deleted"`
	IsEncrypted bool   `json:"encrypted"`
	HardDeleted bool   `json:"hard_deleted"`
	// Raw JSON string, not a nested object — see note below.
	CryptoMeta *string `json:"crypto_meta,omitempty"`

	Content string `json:"content"`
}

type QuotaReservation struct {
	ID           bson.ObjectID `bson:"_id,omitempty"`
	UserID       string        `bson:"user_id"`
	AttachmentID string        `bson:"attachment_id"`
	SizeBytes    int64         `bson:"size_bytes"`
	Status       string        `bson:"status"`
	ExpiresAt    time.Time     `bson:"expires_at"`
	CreatedAt    time.Time     `bson:"created_at"`
}
