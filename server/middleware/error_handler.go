package middleware

import (
	"errors"
	"log/slog"

	"github.com/gofiber/fiber/v3"
)

type AppError struct {
	Code    int
	Message string
}

func (e *AppError) Error() string {
	return e.Message
}

func NotFound(msg string) *AppError     { return &AppError{Code: 404, Message: msg} }
func Unauthorized(msg string) *AppError { return &AppError{Code: 401, Message: msg} }
func BadRequest(msg string) *AppError   { return &AppError{Code: 400, Message: msg} }
func Internal(msg string) *AppError     { return &AppError{Code: 500, Message: msg} }
func Conflict(msg string) *AppError     { return &AppError{Code: 409, Message: msg} }
func ErrorHandler(c fiber.Ctx, err error) error {
	var appErr *AppError
	if errors.As(err, &appErr) {
		return c.Status(appErr.Code).JSON(fiber.Map{
			"error": appErr.Message,
		})
	}

	var fiberErr *fiber.Error
	if errors.As(err, &fiberErr) {
		return c.Status(fiberErr.Code).JSON(fiber.Map{
			"error": fiberErr.Message,
		})
	}

	slog.Error("unexpected error", "err", err)
	return c.Status(500).JSON(fiber.Map{
		"error": "internal server error",
	})
}
