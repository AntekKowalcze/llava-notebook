package config

import (
	"encoding/hex"
	"fmt"
	"os"
)

func GetAccessSecret() ([]byte, error) {
	accessSecret := os.Getenv("JWT_ACCESS_SECRET")
	if accessSecret == "" {
		return nil, fmt.Errorf("Access secrets not set in environment")
	}
	accessSecretBytes := []byte(accessSecret)
	return accessSecretBytes, nil
}

func GetPepperSecret() ([]byte, error) {
	pepperSecret := os.Getenv("AUTH_HMAC_SECRET")
	if pepperSecret == "" {
		return nil, fmt.Errorf("auth HMAC secret is not set")
	}
	pepper, err := hex.DecodeString(pepperSecret)
	if err != nil {
		return nil, fmt.Errorf("invalid auth HMAC secret: %w", err)
	}
	if len(pepper) != 32 {
		return nil, fmt.Errorf("auth HMAC secret must be 32 bytes")
	}
	return pepper, nil
}

func GetRefreshSecret() ([]byte, error) {
	refreshSecret := os.Getenv("JWT_REFRESH_SECRET")
	if refreshSecret == "" {
		return nil, fmt.Errorf("Refresh secrets not set in environment")
	}
	refreshSecretBytes := []byte(refreshSecret)
	return refreshSecretBytes, nil
}

func GetJwtKeys() ([]byte, []byte, error) {
	refreshSecret := os.Getenv("JWT_REFRESH_SECRET")
	accessSecret := os.Getenv("JWT_ACCESS_SECRET")
	if refreshSecret == "" || accessSecret == "" {
		return nil, nil, fmt.Errorf("JWT secrets not set in environment")
	}
	accessSecretBytes := []byte(accessSecret)
	refreshSecretBytes := []byte(refreshSecret)
	return refreshSecretBytes, accessSecretBytes, nil
}
