package middleware

import (
	"io"
	"net/http/httptest"
	"testing"

	"github.com/gofiber/fiber/v3"
)

func clientIPFor(t *testing.T, cfg fiber.Config, forwarded string) string {
	t.Helper()
	app := fiber.New(cfg)
	app.Get("/", func(c fiber.Ctx) error { return c.SendString(ClientIP(c)) })

	req := httptest.NewRequest("GET", "/", nil)
	if forwarded != "" {
		req.Header.Set("X-Forwarded-For", forwarded)
	}
	resp, err := app.Test(req)
	if err != nil {
		t.Fatal(err)
	}
	body, _ := io.ReadAll(resp.Body)
	return string(body)
}

func TestClientIPIgnoresTheClientControlledPartOfXForwardedFor(t *testing.T) {
	trusted := fiber.Config{
		ProxyHeader:      "X-Forwarded-For",
		TrustProxy:       true,
		TrustProxyConfig: fiber.TrustProxyConfig{Proxies: []string{"0.0.0.0"}},
	}

	// The proxy appends the real client (5.6.7.8) after whatever was sent.
	if got := clientIPFor(t, trusted, "1.1.1.1, 5.6.7.8"); got != "5.6.7.8" {
		t.Fatalf("spoofed left-most entry was used: %q", got)
	}
	if got := clientIPFor(t, trusted, "not-an-ip, 5.6.7.8"); got != "5.6.7.8" {
		t.Fatalf("got %q", got)
	}

	// Not behind the configured proxy: the header is ignored entirely.
	untrusted := fiber.Config{ProxyHeader: "X-Forwarded-For", TrustProxy: true}
	if got := clientIPFor(t, untrusted, "1.1.1.1"); got == "1.1.1.1" {
		t.Fatal("header from an untrusted peer must not be used")
	}
}
