package middleware

import (
	"net"
	"strings"

	"github.com/gofiber/fiber/v3"
)

// ClientIP is the address rate limits are keyed on.
//
// Fiber's c.IP() returns the left-most entry of a proxy header such as
// X-Forwarded-For, and that part is whatever the client sent: a client could
// put a new fake address in every request and never hit a limit. Behind a
// trusted proxy the right-most valid entry is used instead, because that one
// was appended by the proxy itself (this assumes one proxy hop; for a single
// value header such as X-Real-IP it is simply that value). Requests that did
// not come through a trusted proxy use the connection's address.
func ClientIP(c fiber.Ctx) string {
	header := c.App().Config().ProxyHeader
	if header == "" || !c.IsProxyTrusted() {
		return c.IP()
	}

	entries := strings.Split(c.Get(header), ",")
	for i := len(entries) - 1; i >= 0; i-- {
		ip := strings.TrimSpace(entries[i])
		if net.ParseIP(ip) != nil {
			return ip
		}
	}

	return c.RequestCtx().RemoteIP().String()
}
