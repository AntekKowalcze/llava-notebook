package routes

import (
	"llava-server/config"
	"llava-server/middleware"

	"github.com/go-playground/validator/v10"
	"github.com/gofiber/fiber/v3"
	"go.mongodb.org/mongo-driver/v2/mongo"
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
	secret, error := config.GetAccessSecret()
	if error != nil {
		return middleware.Internal("Couldnt get secret")
	}
	g := app.Group("/auth")
	g.Post("/register", h.Register)
	g.Post("/login", h.Login)
	g.Post("/refresh", h.Refresh)
	g.Post("/logout", middleware.AuthMiddleware(secret), h.Logout)
	g.Post("/logoutAll", middleware.AuthMiddleware(secret), h.LogoutAll)
	return nil
}
