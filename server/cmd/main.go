package main

import (
	"context"
	"llava-server/config"
	"llava-server/middleware"
	"llava-server/routes"
	"log"
	"log/slog"
	"os"
	"time"

	awsconfig "github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/s3"
	"github.com/go-playground/validator/v10"
	"github.com/gofiber/fiber/v3"
	"github.com/joho/godotenv"
	"google.golang.org/genai"
)

func main() {
	validate := validator.New(validator.WithRequiredStructEnabled())
	ctx := context.Background()
	if err := godotenv.Load(); err != nil {
		slog.Warn("Couldn't load .env file")
	}

	client, err := config.GetMongoConnection()
	if err != nil {
		log.Fatal("Failed to get mongo connection, server can not work without it", err)
	}
	defer client.Disconnect(ctx)

	app := fiber.New(fiber.Config{
		ErrorHandler:  middleware.ErrorHandler,
		CaseSensitive: true,
		StrictRouting: true,
		ServerHeader:  "Llava",
		AppName:       "Llava-dev",
	})

	cfg, err := awsconfig.LoadDefaultConfig(ctx)
	if err != nil {
		log.Fatal("Failed to get config for s3")
	}

	s3Client := s3.NewFromConfig(cfg)

	app.Get("/", func(c fiber.Ctx) error {
		return c.SendString("running")
	})

	syncHandler := routes.NewSyncHandler(
		client.Database("llava"),
		validate,
		s3Client,
		os.Getenv("S3_BUCKET"),
		s3.NewPresignClient(s3Client),
	)
	indexCtx, cancelIndexes := context.WithTimeout(ctx, 10*time.Second)
	defer cancelIndexes()
	if err := syncHandler.EnsureIndexes(indexCtx); err != nil {
		log.Fatal("Failed to create sync indexes", err)
	}

	apiKey := os.Getenv("GEMINI_API_KEY")

	genaiClient, err := genai.NewClient(ctx, &genai.ClientConfig{
		APIKey: apiKey,
	})
	if err != nil {
		log.Fatalf("Failed to initialize Gemini client: %v", err)
	}

	aiHandler := &routes.AiHandler{
		Client:    genaiClient,
		Validator: validate,
	}
	syncHandler.StartReservationCleanupWorker(ctx)
	h := routes.NewHandler(client.Database("llava"), validate)

	if err := aiHandler.RegisterAiRoutes(app); err != nil {
		log.Fatal(err)
	}
	if err := syncHandler.RegisterSyncRoutes(app); err != nil {
		log.Fatal(err)
	}
	if err := h.RegisterJwtRoutes(app); err != nil {
		log.Fatal(err)
	}
	address := os.Getenv("LISTEN_ADDRESS")
	log.Fatal(app.Listen(address))
}
