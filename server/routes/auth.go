package routes

import (
	"context"
	"crypto/hmac"
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"llava-server/config"
	"llava-server/middleware"
	"llava-server/models"
	"strings"
	"time"

	"github.com/gofiber/fiber/v3"
	"github.com/gofiber/fiber/v3/log"
	"github.com/google/uuid"
	"go.mongodb.org/mongo-driver/v2/bson"
	"go.mongodb.org/mongo-driver/v2/mongo"
	"go.mongodb.org/mongo-driver/v2/mongo/options"
)

func (h *Handler) Register(c fiber.Ctx) error {
	registerRequest := new(models.RegisterRequest)

	if err := c.Bind().Body(registerRequest); err != nil {
		return middleware.BadRequest("Couldnt read body data")
	}

	if err := h.Validator.Struct(registerRequest); err != nil {
		return middleware.BadRequest("Wrong user struct was sent")
	}

	pepperSecret, err := config.GetPepperSecret()
	if err != nil {
		log.Errorf("register: failed to get pepper secret: %v", err)
		return middleware.Internal("Internal server configuration error")
	}

	localUser := registerRequest.LocalUserModel

	mac := hmac.New(sha256.New, pepperSecret)
	mac.Write([]byte(localUser.PasswordHash))
	passwordVerifier := mac.Sum(nil)

	user := &models.User{
		ID:               localUser.ID,
		Email:            localUser.Email,
		EmailVerified:    localUser.EmailVerified,
		PasswordVerifier: passwordVerifier,
		PasswordSalt:     localUser.PasswordSalt,
		MasterKeyEnc:     localUser.MasterKeyEnc,
		MasterKeyNonce:   localUser.MasterKeyNonce,
		KekSalt:          localUser.KekSalt,
		ArgonParams:      localUser.ArgonParams,

		StorageUsed:    localUser.StorageUsed,
		QuotaBytes:     localUser.QuotaBytes,
		FailedAttempts: 0,
		LockoutUntil:   nil,

		CreatedAt: time.Now().UnixMilli(),
		LastLogin: time.Now().UnixMilli(),
	}

	device := registerRequest.Device

	ctx, cancel := context.WithTimeout(
		context.Background(),
		5*time.Second,
	)
	defer cancel()

	usersColl := h.DB.Collection("users_data")

	opts := options.FindOne().SetProjection(bson.M{"_id": 1})

	err = usersColl.FindOne(
		ctx,
		bson.M{"email": user.Email},
		opts,
	).Err()

	if err == nil {
		return middleware.Conflict("this email exists")
	}

	if err != mongo.ErrNoDocuments {
		log.Errorf("register: email lookup failed: %v", err)
		return middleware.Internal("database error")
	}

	res, err := usersColl.InsertOne(ctx, user)
	if err != nil {
		log.Errorf("register: insert user failed: %v", err)
		return middleware.Internal("Couldnt insert data to collection")
	}

	userID, ok := res.InsertedID.(bson.ObjectID)
	if !ok {
		log.Errorf("register: inserted ID is not ObjectID")
		return middleware.Internal("database error")
	}

	devicesColl := h.DB.Collection("devices")

	device.UserID = userID
	device.CreatedAt = time.Now().UnixMilli()
	device.LastSeen = time.Now().UnixMilli()

	_, err = devicesColl.InsertOne(ctx, device)
	if err != nil {
		log.Errorf("register: insert device failed: %v", err)
		return middleware.Internal("Couldnt insert data to collection")
	}

	accessToken, err := middleware.GenerateAccessToken(
		device.DeviceID,
		userID.Hex(),
	)
	if err != nil {
		return middleware.Internal("Couldnt generate access token")
	}

	jti, signature, err := middleware.GenerateRefreshToken()
	if err != nil {
		return middleware.Internal("Couldnt generate refresh token")
	}

	jwtColl := h.DB.Collection("jwt")

	refreshInsert := &models.RefreshToken{
		CreatedAt: time.Now().UnixMilli(),
		ExpiresAt: time.Now().Add(30 * 24 * time.Hour).UnixMilli(),
		JTI:       jti.String(),
		UserID:    userID,
		DeviceID:  device.DeviceID,
		TokenHash: signature,
	}

	_, err = jwtColl.InsertOne(ctx, refreshInsert)
	if err != nil {
		log.Errorf("register: insert refresh token failed: %v", err)
		return middleware.Internal("Couldnt insert data to collection")
	}

	return c.Status(fiber.StatusCreated).JSON(fiber.Map{
		"access_token":  accessToken,
		"refresh_token": jti.String() + "." + signature,
		"user_id":       userID.Hex(),
	})
}

type PreLoginRequest struct {
	Email string `json:"email"  validate:"required,email"`
}

func generateDummySalt() []byte {
	salt := make([]byte, 16)
	rand.Read(salt)
	return salt
}
func (h *Handler) PreLogin(c fiber.Ctx) error {
	request := new(PreLoginRequest)

	if err := c.Bind().Body(request); err != nil {
		return middleware.BadRequest("no body")
	}

	if err := h.Validator.Struct(request); err != nil {
		return middleware.BadRequest("Wrong pre login struct was sent")
	}

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	usersColl := h.DB.Collection("users_data")

	var user models.User

	err := usersColl.FindOne(
		ctx,
		bson.M{"email": request.Email},
	).Decode(&user)

	if err != nil {
		// Nie ujawniamy, czy email istnieje.
		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"password_salt": base64.RawStdEncoding.EncodeToString(generateDummySalt()),
		})
	}

	return c.Status(fiber.StatusOK).JSON(fiber.Map{
		"password_salt": user.PasswordSalt,
	})
}

type LoginRequest struct {
	Email        string    `json:"email"  validate:"required,email"`
	PasswordHash string    `json:"password_hash" validate:"required"`
	DeviceID     uuid.UUID `json:"device_id"  validate:"required"`
}

func (h *Handler) Login(c fiber.Ctx) error {
	var request = new(LoginRequest)
	err := c.Bind().Body(request)
	if err != nil {
		return middleware.BadRequest("no body")
	}
	if err := h.Validator.Struct(request); err != nil {
		return middleware.BadRequest("Wrong user struct was sent")
	}
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	usersColl := h.DB.Collection("users_data")
	res := usersColl.FindOne(ctx, bson.M{"email": request.Email})
	var user models.User

	if err := res.Decode(&user); err != nil {
		return middleware.Unauthorized("invalid_credentials")
	}

	deviceColl := h.DB.Collection("devices")

	res = deviceColl.FindOne(ctx, bson.M{"user_id": user.ID, "device_id": request.DeviceID})
	var device models.Device
	if err := res.Decode(&device); err != nil {
		return middleware.Unauthorized("invalid_credentials")
	}

	now := time.Now().UnixMilli()
	if user.LockoutUntil != nil && now < *user.LockoutUntil {
		return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{
			"timeout": *user.LockoutUntil,
		})
	}
	pepperSecret, err := config.GetPepperSecret()
	if err != nil {
		log.Errorf("register: failed to get pepper secret: %v", err)
		return middleware.Internal("Internal server configuration error")
	}
	mac := hmac.New(sha256.New, pepperSecret)
	mac.Write([]byte(request.PasswordHash))
	passwordVerifier := mac.Sum(nil)

	if subtle.ConstantTimeCompare([]byte(user.PasswordVerifier), []byte(passwordVerifier)) == 1 {
		accessToken, err := middleware.GenerateAccessToken(device.DeviceID, user.ID.Hex())
		if err != nil {
			return middleware.Internal("Couldnt generate Access Token")
		}
		jti, signature, err := middleware.GenerateRefreshToken()
		if err != nil {
			return middleware.Internal("Couldnt generate Refresh Token")
		}

		jwtColl := h.DB.Collection("jwt")
		refreshInsert := new(models.RefreshToken)
		refreshInsert.CreatedAt = time.Now().UnixMilli()
		refreshInsert.ExpiresAt = time.Now().Add(30 * 24 * time.Hour).UnixMilli()
		refreshInsert.JTI = jti.String()
		refreshInsert.UserID = *user.ID
		refreshInsert.DeviceID = device.DeviceID
		refreshInsert.TokenHash = signature

		if _, err = jwtColl.InsertOne(ctx, refreshInsert); err != nil {
			return middleware.Internal("couldn't save refresh token")
		}
		usersColl.UpdateOne(ctx, bson.M{"_id": user.ID}, bson.M{
			"$set": bson.M{
				"last_login":      time.Now().UnixMilli(),
				"failed_attempts": 0,
			},
		})
		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"access_token":     accessToken,
			"refresh_token":    jti.String() + "." + signature,
			"user_id":          refreshInsert.UserID.Hex(),
			"master_key_enc":   base64.StdEncoding.EncodeToString(user.MasterKeyEnc),
			"master_key_nonce": base64.StdEncoding.EncodeToString(user.MasterKeyNonce),
			"kek_salt":         user.KekSalt,
			"params":           user.ArgonParams,
		})

	} else {
		failedAttempts := user.FailedAttempts + 1
		if _, err := usersColl.UpdateOne(ctx, bson.M{"_id": user.ID}, bson.M{
			"$set": bson.M{
				"failed_attempts": failedAttempts,
			},
		}); err != nil {
			return middleware.Unauthorized("wrong password")
		}

		if (user.FailedAttempts+1)%5 == 0 {
			timeoutUntil := time.Now().Add(30 * time.Second).UnixMilli()
			_, err = usersColl.UpdateOne(ctx, bson.M{"_id": user.ID}, bson.M{"$set": bson.M{"lockout_until": timeoutUntil}})
			if err != nil {
				return middleware.Internal("Internal Error")
			}
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{
				"timeout": timeoutUntil,
			})
		}

		return middleware.Unauthorized("wrong password")
	}
}

type RefreshRequest struct {
	RefreshToken string `json:"refresh_token"`
}

func (h *Handler) Refresh(c fiber.Ctx) error {
	req := new(RefreshRequest)
	if err := c.Bind().Body(req); err != nil {
		return middleware.BadRequest("invalid refresh token")
	}

	parts := strings.SplitN(req.RefreshToken, ".", 2)
	if len(parts) != 2 {
		return middleware.Unauthorized("invalid_refresh_token")
	}
	jtiParsed, err := uuid.Parse(parts[0])
	if err != nil {
		return middleware.Unauthorized("invalid_refresh_token")
	}
	signature := parts[1]

	jwtColl := h.DB.Collection("jwt")
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	now := time.Now().UnixMilli()
	res := jwtColl.FindOneAndDelete(ctx, bson.M{
		"jti":        jtiParsed.String(),
		"token_hash": signature,
		"expires_at": bson.M{"$gt": now},
	})

	var refresh models.RefreshToken
	if err := res.Decode(&refresh); err != nil {
		if err != mongo.ErrNoDocuments {
			return middleware.Internal("database error")
		}

		expiredRes := jwtColl.FindOneAndDelete(ctx, bson.M{
			"jti":        jtiParsed.String(),
			"token_hash": signature,
			"expires_at": bson.M{"$lte": now},
		})
		if expiredRes.Err() == nil {
			return middleware.Unauthorized("session_expired")
		}
		if expiredRes.Err() != mongo.ErrNoDocuments {
			return middleware.Internal("database error")
		}
		return middleware.Unauthorized("invalid_refresh_token")
	}

	accessToken, err := middleware.GenerateAccessToken(refresh.DeviceID, refresh.UserID.Hex())
	if err != nil {
		return middleware.Internal("Couldnt generate new access token")
	}
	jtis, signature, err := middleware.GenerateRefreshToken()
	if err != nil {
		return middleware.Internal("Couldnt generate new refresh token")
	}

	var newRefresh models.RefreshToken
	newRefresh.CreatedAt = time.Now().UnixMilli()
	newRefresh.ExpiresAt = time.Now().Add(30 * 24 * time.Hour).UnixMilli()
	newRefresh.UserID = refresh.UserID
	newRefresh.DeviceID = refresh.DeviceID
	newRefresh.JTI = jtis.String()
	newRefresh.TokenHash = signature
	_, err = jwtColl.InsertOne(ctx, newRefresh)
	if err != nil {
		return middleware.Internal("Couldnt insert new refresh token")
	}

	return c.Status(fiber.StatusOK).JSON(fiber.Map{
		"access_token":  accessToken,
		"refresh_token": jtis.String() + "." + signature,
	})

}

func (h *Handler) Logout(c fiber.Ctx) error {
	userIdStr, ok := c.Locals("userID").(string)
	if !ok {
		return middleware.BadRequest("user not attached")
	}
	deviceId, ok := c.Locals("deviceID").(uuid.UUID)
	if !ok {
		return middleware.BadRequest("device not attached")
	}
	userId, err := bson.ObjectIDFromHex(userIdStr)
	if err != nil {
		return middleware.BadRequest("invalid user id")
	}
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	jwtColl := h.DB.Collection("jwt")
	_, err = jwtColl.DeleteMany(ctx, bson.M{"user_id": userId, "device_id": deviceId})
	if err != nil {
		return middleware.Internal("Couldnt delete refresh token")
	}
	return c.SendStatus(fiber.StatusNoContent)

}

func (h *Handler) LogoutAll(c fiber.Ctx) error {
	userIdStr, ok := c.Locals("userID").(string)
	if !ok {
		return middleware.BadRequest("user not attached")
	}

	userId, err := bson.ObjectIDFromHex(userIdStr)
	if err != nil {
		return middleware.BadRequest("invalid user id")
	}
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	jwtColl := h.DB.Collection("jwt")
	_, err = jwtColl.DeleteMany(ctx, bson.M{"user_id": userId})
	if err != nil {
		return middleware.Internal("Couldnt delete refresh tokens")
	}
	return c.SendStatus(fiber.StatusNoContent)
}
