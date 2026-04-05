package middleware

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"errors"
	"fmt"
	"llava-server/config"
	"llava-server/models"
	"strings"
	"time"

	"github.com/gofiber/fiber/v3"
	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
)

func GenerateAccessToken(deviceId uuid.UUID, userId string) (string, error) {
	accessSecret, error := config.GetAccessSecret()
	if error != nil {
		return "", fmt.Errorf("Cannot get access secret")
	}
	payload := &models.AccessTokenPayload{
		RegisteredClaims: jwt.RegisteredClaims{
			Subject:   userId,
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(15 * time.Minute)),
			IssuedAt:  jwt.NewNumericDate(time.Now()),
			Audience:  jwt.ClaimStrings{"llava-server"},
		},
		DeviceID: deviceId,
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS512, payload)
	tokenString, err := token.SignedString(accessSecret)
	if err != nil {
		return "", fmt.Errorf("Cannot create token")
	}
	return tokenString, nil
}

func GenerateRefreshToken() (uuid.UUID, string, error) {
	refreshSecret, error := config.GetRefreshSecret()
	if error != nil {
		return uuid.Nil, "", fmt.Errorf("Cannot get refresh secret")
	}
	jti := uuid.New()

	mac := hmac.New(sha256.New, refreshSecret)
	mac.Write([]byte(jti.String()))
	signature := base64.StdEncoding.EncodeToString(mac.Sum(nil))

	return jti, signature, nil
}
func ValidateAccessToken(tokenString string, accessSecret []byte) (*models.AccessTokenPayload, error) {
	payload := &models.AccessTokenPayload{}

	token, err := jwt.ParseWithClaims(tokenString, payload, func(token *jwt.Token) (interface{}, error) {
		if token.Method != jwt.SigningMethodHS512 {
			return nil, fmt.Errorf("unexpected signing method: %v", token.Header["alg"])
		}
		return accessSecret, nil
	}, jwt.WithAudience("llava-server"))

	if err != nil {
		return nil, err
	}
	if !token.Valid {
		return nil, fmt.Errorf("invalid token")
	}

	return payload, nil
}

// TODO 1 parse jti in refresh to uuid from string
func AuthMiddleware(accessSecret []byte) fiber.Handler {
	return func(c fiber.Ctx) error {
		tokenString := strings.TrimPrefix(c.Get(fiber.HeaderAuthorization), "Bearer ")
		if tokenString == "" {
			return Unauthorized("missing_token")
		}
		payload, err := ValidateAccessToken(tokenString, accessSecret)
		if err != nil {
			if errors.Is(err, jwt.ErrTokenExpired) {
				return Unauthorized("token_expired")
			}
			return Unauthorized("invalid_token")
		}
		c.Locals("userID", payload.Subject)
		c.Locals("deviceID", payload.DeviceID)
		return c.Next()
	}
}
