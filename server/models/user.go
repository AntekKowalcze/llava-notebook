package models

import (
	"github.com/google/uuid"
	"go.mongodb.org/mongo-driver/v2/bson"
)

type RegisterRequest struct {
	User   User   `json:"user"   validate:"required"`
	Device Device `json:"device" validate:"required"`
}

type User struct {
	ID             *bson.ObjectID `bson:"_id,omitempty"      json:"_id,omitempty"`
	Email          string         `bson:"email"              json:"email"              validate:"required,email"`
	EmailVerified  bool           `bson:"email_verified"     json:"email_verified"`
	PasswordHash   string         `bson:"password_hash"      json:"password_hash"      validate:"required"`
	MasterKeyEnc   []byte         `bson:"master_key_enc"     json:"master_key_enc"     validate:"required"`
	MasterKeyNonce []byte         `bson:"master_key_nonce"   json:"master_key_nonce"   validate:"required"`
	KekSalt        string         `bson:"kek_salt"           json:"kek_salt"           validate:"required"`
	ArgonParams    ArgonParams    `bson:"argon2_params"      json:"argon2_params"      validate:"required"`
	StorageUsed    int64          `bson:"storage_used_bytes" json:"storage_used_bytes"`
	QuotaBytes     int64          `bson:"quota_bytes"        json:"quota_bytes"`
	FailedAttempts int64          `bson:"failed_attempts"    json:"failed_attempts"`
	LockoutUntil   *int64         `bson:"lockout_until"      json:"lockout_until"`
	CreatedAt      int64          `bson:"created_at"         json:"created_at"`
	LastLogin      int64          `bson:"last_login"         json:"last_login"`
}

type ArgonParams struct {
	MCost uint32 `bson:"m_cost" json:"m_cost" validate:"required,gte=1"`
	TCost uint32 `bson:"t_cost" json:"t_cost" validate:"required,gte=1"`
	PCost uint32 `bson:"p_cost" json:"p_cost" validate:"required,gte=1"`
}

type Device struct {
	ID         *bson.ObjectID `bson:"_id,omitempty" json:"_id,omitempty"`
	DeviceID   uuid.UUID      `bson:"device_id"     json:"device_id"     validate:"required,uuid"`
	UserID     bson.ObjectID  `bson:"user_id"       json:"user_id"`
	DeviceName string         `bson:"device_name"   json:"device_name"   validate:"required"`
	LastSeen   int64          `bson:"last_seen"     json:"last_seen"`
	CreatedAt  int64          `bson:"created_at"    json:"created_at"`
}
