package main

import (
	"context"
	"fmt"
	"llava-server/config"
	"llava-server/middleware"
	"llava-server/routes"
	"log"
	"log/slog"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	awsconfig "github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/s3"
	"github.com/go-playground/validator/v10"
	"github.com/gofiber/fiber/v3"
	"github.com/joho/godotenv"
	"google.golang.org/genai"
)

func main() {
	if err := run(); err != nil {
		log.Fatalf("server stopped: %v", err)
	}
}

// run holds the server's lifetime so that deferred cleanup (Mongo disconnect)
// actually executes; log.Fatal in the old main skipped it.
func run() error {
	// Cancelled on SIGINT/SIGTERM. Also stops the reservation cleanup worker.
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	validate := validator.New(validator.WithRequiredStructEnabled())
	if err := godotenv.Load(); err != nil {
		slog.Warn("Couldn't load .env file")
	}

	client, err := config.GetMongoConnection()
	if err != nil {
		return fmt.Errorf("failed to get mongo connection, server can not work without it: %w", err)
	}
	defer func() {
		disconnectCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		if err := client.Disconnect(disconnectCtx); err != nil {
			slog.Error("failed to disconnect from mongo", "error", err)
		}
	}()

	fiberConfig := fiber.Config{
		ErrorHandler:  middleware.ErrorHandler,
		CaseSensitive: true,
		StrictRouting: true,
		ServerHeader:  "Llava",
		AppName:       "Llava-dev",
	}
	// Behind a reverse proxy c.IP() is the proxy's address, which would put
	// every user into the same rate-limit bucket. Opt in with PROXY_HEADER
	// (X-Real-IP, or X-Forwarded-For when exactly one proxy appends to it);
	// TRUSTED_PROXIES is a comma-separated list of proxy IPs/CIDRs,
	// defaulting to loopback and private ranges. Rate limits key on
	// middleware.ClientIP, which never trusts the client-sent part of the
	// header.
	if header := os.Getenv("PROXY_HEADER"); header != "" {
		fiberConfig.ProxyHeader = header
		fiberConfig.TrustProxy = true
		if proxies := splitList(os.Getenv("TRUSTED_PROXIES")); len(proxies) > 0 {
			fiberConfig.TrustProxyConfig = fiber.TrustProxyConfig{Proxies: proxies}
		} else {
			fiberConfig.TrustProxyConfig = fiber.TrustProxyConfig{Loopback: true, Private: true}
		}
	}
	app := fiber.New(fiberConfig)

	cfg, err := awsconfig.LoadDefaultConfig(ctx)
	if err != nil {
		return fmt.Errorf("failed to get config for s3: %w", err)
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
		return fmt.Errorf("failed to create sync indexes: %w", err)
	}

	apiKey := os.Getenv("GEMINI_API_KEY")

	genaiClient, err := genai.NewClient(ctx, &genai.ClientConfig{
		APIKey: apiKey,
	})
	if err != nil {
		return fmt.Errorf("failed to initialize Gemini client: %w", err)
	}

	aiHandler := &routes.AiHandler{
		Client:    genaiClient,
		Validator: validate,
	}
	syncHandler.StartReservationCleanupWorker(ctx)
	h := routes.NewHandler(client.Database("llava"), validate)

	if err := aiHandler.RegisterAiRoutes(app); err != nil {
		return err
	}
	if err := syncHandler.RegisterSyncRoutes(app); err != nil {
		return err
	}
	if err := h.RegisterJwtRoutes(app); err != nil {
		return err
	}

	address := os.Getenv("LISTEN_ADDRESS")
	listenErr := make(chan error, 1)
	go func() { listenErr <- app.Listen(address) }()

	select {
	case err := <-listenErr:
		return err
	case <-ctx.Done():
	}

	slog.Info("shutting down, waiting for in-flight requests")
	// Sync handlers run with 50s timeouts; give them room to finish.
	shutdownCtx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()
	return app.ShutdownWithContext(shutdownCtx)
}

func splitList(value string) []string {
	var out []string
	for _, item := range strings.Split(value, ",") {
		if item = strings.TrimSpace(item); item != "" {
			out = append(out, item)
		}
	}
	return out
}
