package models

import (
	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
	"go.mongodb.org/mongo-driver/v2/bson"
)

type RefreshToken struct {
	ID        *bson.ObjectID `bson:"_id,omitempty" json:"_id,omitempty"`
	UserID    bson.ObjectID  `bson:"user_id"       json:"user_id" validate:"required"`
	DeviceID  uuid.UUID      `bson:"device_id" json:"device_id" validate:"required"`
	TokenHash string         `bson:"token_hash"    json:"-"`
	JTI       uuid.UUID      `bson:"jti"           json:"jti"`
	CreatedAt int64          `bson:"created_at"    json:"created_at"`
	ExpiresAt int64          `bson:"expires_at"    json:"expires_at"`
}

type AccessTokenPayload struct {
	jwt.RegisteredClaims
	DeviceID uuid.UUID `json:"device_id" validate:"required,uuid"`
}
