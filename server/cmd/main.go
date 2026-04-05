package main

import (
	"context"
	"log"
	"log/slog"

	"github.com/go-playground/validator/v10"
	"github.com/gofiber/fiber/v3"
	"github.com/joho/godotenv"

	"llava-server/config"
	"llava-server/middleware"
	"llava-server/routes"
)

func main() {
	validate := validator.New(validator.WithRequiredStructEnabled())

	if err := godotenv.Load(); err != nil {
		slog.Warn("Couldn't load .env file")
	}

	client, err := config.GetMongoConnection()
	if err != nil {
		log.Fatal("Failed to get mongo connection, server can not work without it", err)
	}
	defer client.Disconnect(context.Background())

	app := fiber.New(fiber.Config{
		ErrorHandler:  middleware.ErrorHandler,
		CaseSensitive: true,
		StrictRouting: true,
		ServerHeader:  "Llava",
		AppName:       "Llava-dev",
	})

	app.Get("/", func(c fiber.Ctx) error {
		return c.SendString("running")
	})
	h := routes.NewHandler(client.Database("llava"), validate) // when should i get this?
	h.RegisterJwtRoutes(app)
	log.Fatal(app.Listen(":3000"))
}
