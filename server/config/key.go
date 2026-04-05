package config

import (
	"fmt"
	"log/slog"
	"os"
)

func GetAccessSecret() ([]byte, error) {
	accessSecret := os.Getenv("JWT_ACCESS_SECRET")
	if accessSecret == "" {
		slog.Error("keys should not be empty string")
		return nil, fmt.Errorf("Access secrets not set in environment")
	}
	accessSecretBytes := []byte(accessSecret)
	return accessSecretBytes, nil
}

func GetRefreshSecret() ([]byte, error) {
	refreshSecret := os.Getenv("JWT_REFRESH_SECRET")
	if refreshSecret == "" {
		slog.Error("keys should not be empty string")
		return nil, fmt.Errorf("Refresh secrets not set in environment")
	}
	refreshSecretBytes := []byte(refreshSecret)
	return refreshSecretBytes, nil
}

func GetJwtKeys() ([]byte, []byte, error) {
	refreshSecret := os.Getenv("JWT_REFRESH_SECRET")
	accessSecret := os.Getenv("JWT_ACCESS_SECRET")
	if refreshSecret == "" || accessSecret == "" {
		slog.Error("keys should not be empty string")
		return nil, nil, fmt.Errorf("JWT secrets not set in environment")
	}
	accessSecretBytes := []byte(accessSecret)
	refreshSecretBytes := []byte(refreshSecret)
	return refreshSecretBytes, accessSecretBytes, nil
}
