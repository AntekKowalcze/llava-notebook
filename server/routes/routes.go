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
	"go.mongodb.org/mongo-driver/v2/mongo"
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
