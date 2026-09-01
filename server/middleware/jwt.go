// Package middleware provides HTTP middleware and authentication utilities for
// protecting server endpoints and managing access and refresh credentials.
//
// Purpose:
//
// This package handles authentication at the HTTP middleware boundary. It
// generates and validates short-lived JWT access tokens, generates refresh
// token credentials, and attaches authenticated user and device identifiers
// to the request context for downstream handlers.
//
// Exports:
//
//   - GenerateAccessToken — Creates a signed HS512 JWT access token containing
//     the authenticated user, device identifier, audience, issue time, and
//     expiration time.
//   - GenerateRefreshToken — Generates a new refresh-token identifier and
//     creates its HMAC-SHA256 signature using the configured refresh secret.
//   - ValidateAccessToken — Parses and validates an access token, including
//     its signing algorithm, signature, registered claims, and audience.
//   - AuthMiddleware — Fiber middleware that extracts a Bearer token,
//     validates it, and stores the authenticated user and device identifiers
//     in the request context.
//
// Key design decisions:
//
// Access tokens are short-lived JWTs with a 15-minute expiration period. They
// are signed using HMAC-SHA512 and include the user identifier as the subject,
// the device identifier as a custom claim, and a fixed server audience.
//
// Token validation explicitly checks that the received signing method is
// HMAC-SHA512 before the configured access secret is used to verify the
// signature. Audience validation is also enforced during JWT parsing.
//
// Refresh tokens are represented by a randomly generated UUIDv4 identifier.
// The identifier itself is not returned as a signed JWT; instead, an
// HMAC-SHA256 value derived from the configured refresh secret and the UUID is
// generated alongside it. This keeps refresh-token credentials independent
// from access-token JWTs.
//
// Authentication middleware stores only the validated user and device
// identifiers in Fiber's request-local storage. Downstream handlers can
// therefore consume authenticated identity information without reparsing the
// token.
//
// Authentication failures are translated into application-level HTTP 401
// responses. Expired access tokens are distinguished from otherwise invalid
// tokens so callers can react appropriately.
//
// Secrets are obtained from the application configuration layer rather than
// being embedded in this package or stored in generated credentials.
//
// Dependencies:
//
//   - github.com/gofiber/fiber/v3 — HTTP middleware, request context, and
//     unauthorized responses.
//   - github.com/golang-jwt/jwt/v5 — JWT creation, signing, parsing, and claim
//     validation.
//   - github.com/google/uuid — Generation and handling of device and refresh
//     token identifiers.
//   - crypto/hmac — HMAC construction for refresh-token signatures.
//   - crypto/sha256 — SHA-256 digest used by refresh-token signing.
//   - encoding/base64 — Encoding of the refresh-token HMAC output.
//   - llava-server/config — Retrieval of access and refresh secrets.
//   - llava-server/models — Definition of access-token claims.
//   - errors — Comparison of JWT validation errors such as expiration.
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
