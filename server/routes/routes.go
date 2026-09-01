// Package routes provides HTTP handler types, constructors, route registration,
// request limiting, and shared resource coordination for the server API.
//
// Purpose:
//
// This package defines the server-side handler objects used by the
// authentication, synchronization, and AI endpoints. It is responsible for
// connecting application services to Fiber routers, configuring endpoint
// authentication and rate limits, and controlling concurrent access to shared
// external resources.
//
// The package contains separate handlers for account authentication,
// synchronization with cloud storage, and AI requests. Synchronization
// handlers additionally coordinate access to MongoDB and S3 through a bounded
// worker semaphore.
//
// Exports:
//
//   - Handler — Holds the MongoDB and validation dependencies required by
//     authentication and account-related routes.
//   - NewHandler — Creates a Handler with the users_data MongoDB collection
//     and shared request validator.
//   - RegisterJwtRoutes — Registers authentication endpoints under /auth and
//     applies authentication-specific rate limiting.
//   - SyncHandler — Holds MongoDB, S3, presigning, validation, and concurrency
//     dependencies required by synchronization routes.
//   - NewSyncHandler — Creates a SyncHandler and initializes its bounded
//     worker semaphore.
//   - EnsureIndexes — Creates the unique MongoDB index required to make note
//     uploads idempotent per user and local note identifier.
//   - RegisterSyncRoutes — Registers authenticated synchronization endpoints
//     under /sync and applies per-user request limits.
//   - AiHandler — Holds the validation and Google AI client dependencies
//     required by AI endpoints.
//   - NewAiHandler — Creates an AiHandler with the Google AI client and
//     request validator.
//   - RegisterAiRoutes — Registers the authenticated AI endpoint under /ai
//     and applies per-user request limiting.
//
// Key design decisions:
//
// Handler types are separated by application responsibility rather than
// sharing one large handler structure. This keeps authentication, cloud
// synchronization, and AI dependencies isolated and makes route registration
// explicit.
//
// Authentication is applied at the route-group level where appropriate.
// Synchronization endpoints are protected by the access-token middleware
// before request-specific handlers are reached, while logout and AI routes
// explicitly attach the same middleware to the protected endpoints.
//
// Rate limiting uses different policies for different classes of operations.
// Authentication requests are limited by client IP, while authenticated
// synchronization and AI requests are primarily limited by user identifier.
// This prevents a single authenticated account from exhausting resources
// independently of the network address it uses.
//
// Synchronization writes and AI requests are intentionally allowed higher
// limits than authentication attempts because they represent normal
// application activity rather than credential-guessing operations.
//
// SyncHandler uses a bounded semaphore to limit concurrent operations against
// external resources such as MongoDB and S3. Callers acquire a worker slot
// before performing a bounded external operation and release it afterwards.
// Acquisition respects the request context so cancelled or timed-out
// operations do not remain blocked indefinitely.
//
// MongoDB note writes are made idempotent with a unique compound index on
// owner_id and local_id. The partial filter preserves compatibility with
// documents created before local_id was introduced.
//
// S3 presigned URLs are managed by SyncHandler so attachment data can be
// transferred directly between clients and S3 without routing file contents
// through the application server.
//
// Secrets required for access-token validation are obtained from the
// configuration layer during route registration rather than being stored in
// handler instances.
//
// ## Dependencies
//
//   - github.com/gofiber/fiber/v3 — HTTP routing, request handling, and
//     response generation.
//   - github.com/gofiber/fiber/v3/middleware/limiter — Endpoint and
//     user-specific rate limiting.
//   - go.mongodb.org/mongo-driver/v2 — MongoDB collections, indexes, and
//     database access.
//   - github.com/aws/aws-sdk-go-v2/service/s3 — S3 client and presigned
//     attachment operations.
//   - github.com/go-playground/validator/v10 — Request validation.
//   - google.golang.org/genai — Google AI client used by AiHandler.
//   - llava-server/config — Retrieval of authentication secrets.
//   - llava-server/middleware — Access-token authentication and standardized
//     HTTP error responses.
package routes

import (
	"context"
	"llava-server/config"
	"llava-server/middleware"
	"time"

	"github.com/aws/aws-sdk-go-v2/service/s3"
	"github.com/go-playground/validator/v10"
	"github.com/gofiber/fiber/v3"
	"github.com/gofiber/fiber/v3/middleware/limiter"
	"go.mongodb.org/mongo-driver/v2/bson"
	"go.mongodb.org/mongo-driver/v2/mongo"
	"go.mongodb.org/mongo-driver/v2/mongo/options"
	"google.golang.org/genai"
)

type Handler struct { //some datatype which holds connections
	DB        *mongo.Database
	Coll      *mongo.Collection
	Validator *validator.Validate
}

func NewHandler(db *mongo.Database, v *validator.Validate) *Handler {
	return &Handler{DB: db, Coll: db.Collection("users_data"), Validator: v}
} // somehting like init of this Hanlder

func (h *Handler) RegisterJwtRoutes(app fiber.Router) error { //this is "receiver" method like thing
	secret, err := config.GetAccessSecret()
	if err != nil {
		return middleware.Internal("Couldnt get secret")
	}

	g := app.Group("/auth")
	g.Use(limiter.New(limiter.Config{
		Max:        10,
		Expiration: time.Minute,
		KeyGenerator: func(c fiber.Ctx) string {
			return "ip:" + c.IP()
		},
		LimitReached: func(c fiber.Ctx) error {
			return c.Status(fiber.StatusTooManyRequests).JSON(fiber.Map{
				"error": "too many authentication requests, slow down",
			})

		},
	}))
	g.Post("/register", h.Register)
	g.Post("/pre-login", h.PreLogin)
	g.Post("/login", h.Login)
	g.Post("/refresh", h.Refresh)
	g.Post("/logout", middleware.AuthMiddleware(secret), h.Logout)
	g.Post("/logoutAll", middleware.AuthMiddleware(secret), h.LogoutAll)
	return nil
}

type SyncHandler struct {
	DB        *mongo.Database
	Coll      *mongo.Collection
	Validator *validator.Validate
	s3Client  *s3.Client
	s3Bucket  string
	presigner *s3.PresignClient
	workerSem chan struct{}
}

func NewSyncHandler(db *mongo.Database, v *validator.Validate, s3Client *s3.Client, s3Bucket string, presigner *s3.PresignClient) *SyncHandler {
	return &SyncHandler{DB: db, Coll: db.Collection("notes"), Validator: v, s3Client: s3Client, s3Bucket: s3Bucket, presigner: presigner, workerSem: make(chan struct{}, 50)}

}

// EnsureIndexes installs the constraint that makes upload-note idempotent for
// a given user's local note ID. The partial filter keeps existing documents
// created before local_id was stored out of the unique index.
func (s *SyncHandler) EnsureIndexes(ctx context.Context) error {
	_, err := s.Coll.Indexes().CreateOne(ctx, mongo.IndexModel{
		Keys: bson.D{{Key: "owner_id", Value: 1}, {Key: "local_id", Value: 1}},
		Options: options.Index().
			SetName("owner_local_id_unique").
			SetUnique(true).
			SetPartialFilterExpression(bson.M{"local_id": bson.M{"$exists": true}}),
	})
	return err
}

func (s *SyncHandler) acquireWorker(ctx context.Context) error {
	select {
	case s.workerSem <- struct{}{}:
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}

func (s *SyncHandler) releaseWorker() {
	<-s.workerSem
}

func (s *SyncHandler) RegisterSyncRoutes(app fiber.Router) error {
	secret, err := config.GetAccessSecret()

	if err != nil {
		return middleware.Internal("Couldnt get secret")
	}

	g := app.Group("/sync")

	g.Use(middleware.AuthMiddleware(secret))

	// 4 sync-check requests / minute / user
	syncCheckLimiter := limiter.New(limiter.Config{
		Max:        10,
		Expiration: time.Minute,

		KeyGenerator: func(c fiber.Ctx) string {
			if userID, ok := c.Locals("userID").(string); ok && userID != "" {
				return "user:" + userID
			}

			return "ip:" + c.IP()
		},

		LimitReached: func(c fiber.Ctx) error {
			return c.Status(fiber.StatusTooManyRequests).JSON(fiber.Map{
				"error": "too many sync-check requests, slow down",
			})
		},
	})

	// 100 upload/update requests / minute / user
	syncWriteLimiter := limiter.New(limiter.Config{
		Max:        100,
		Expiration: time.Minute,

		KeyGenerator: func(c fiber.Ctx) string {
			if userID, ok := c.Locals("userID").(string); ok && userID != "" {
				return "user:" + userID
			}

			return "ip:" + c.IP()
		},

		LimitReached: func(c fiber.Ctx) error {
			return c.Status(fiber.StatusTooManyRequests).JSON(fiber.Map{
				"error": "too many sync requests, slow down",
			})
		},
	})

	g.Post("/sync-check", syncCheckLimiter, s.SyncCheck)
	g.Post("/upload-note", syncWriteLimiter, s.UploadNote)
	g.Put("/update-note/:mongo_id", syncWriteLimiter, s.UpdateNote)
	g.Post("/upload-compleated/:attachment_id", syncWriteLimiter, s.manageReservationAndQuota)
	return nil
}

type AiHandler struct {
	Validator *validator.Validate
	Client    *genai.Client
}

func NewAiHandler(client *genai.Client, v *validator.Validate) AiHandler {
	return AiHandler{Client: client, Validator: v}
}

func (a *AiHandler) RegisterAiRoutes(app fiber.Router) error {
	secret, err := config.GetAccessSecret()
	if err != nil {
		return middleware.Internal("Couldnt get secret")
	}
	g := app.Group("/ai")
	g.Post("/", middleware.AuthMiddleware(secret), limiter.New(limiter.Config{
		Max:        10,
		Expiration: time.Minute,
		KeyGenerator: func(c fiber.Ctx) string {
			if userID, ok := c.Locals("userID").(string); ok && userID != "" {
				return "user:" + userID
			}

			return "ip:" + c.IP()
		},
		LimitReached: func(c fiber.Ctx) error {
			return c.Status(fiber.StatusTooManyRequests).JSON(fiber.Map{
				"error": "You are doing to many ai requests slow down",
			})
		},
	}), a.Ai)

	return nil
}
