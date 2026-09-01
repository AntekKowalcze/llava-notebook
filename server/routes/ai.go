package routes

import (
	"bufio"
	"fmt"
	"log"

	"github.com/gofiber/fiber/v3"
	"google.golang.org/genai"

	"llava-server/middleware"
	"llava-server/models"
)

func (a *AiHandler) Ai(c fiber.Ctx) error {
	var req = new(models.GoogleAiRequest)

	if err := c.Bind().Body(req); err != nil {
		return middleware.BadRequest("no body")
	}

	if err := a.Validator.Struct(req); err != nil {
		return middleware.BadRequest("Wrong user struct was sent")
	}

	ctx := c.Context()

	systemPrompt := `You are an AI assistant integrated into a Markdown note editor.

Your task is to help the user edit, transform, improve, summarize, or generate content inside their current Markdown document.

Rules:

1. Always follow the user's instruction precisely.
2. Use the provided document as context when it is relevant.
3. When a selection is provided, treat it as the primary target of the user's instruction unless the user explicitly asks to modify or consider the whole document.
4. Preserve the existing meaning and structure unless the user asks for a transformation.
5. Preserve Markdown syntax and formatting whenever possible.
6. Return ONLY the content that should be inserted into the editor.
7. Do NOT add explanations, introductions, conclusions, comments, or phrases such as "Here is the result:".
8. Do NOT wrap the response in Markdown code fences.
9. Do NOT output JSON, XML, metadata, or any other wrapper format.
10. Do not invent information that is not supported by the document or the user's instruction.
11. Preserve links, lists, headings, code, tables, and other Markdown structures unless the instruction requires changing them.
12. If the user asks to rewrite or improve text, return the rewritten text directly.
13. If the user asks to generate new content, return only the generated Markdown content.
14. If the user's instruction is ambiguous, make the most reasonable interpretation based on the document and selection.

The output must always be valid Markdown suitable for direct insertion into the editor.`

	config := &genai.GenerateContentConfig{
		SystemInstruction: genai.NewContentFromText(
			systemPrompt,
			genai.RoleUser,
		),
	}

	// The system prompt expects "document" and "selection" as distinct,
	// labeled context — build the actual content from all three fields
	// instead of a single flat prompt string.
	userContent := fmt.Sprintf(
		"DOCUMENT:\n%s\n\nSELECTION:\n%s\n\nINSTRUCTION:\n%s",
		req.Document,
		req.Selection,
		req.Prompt,
	)

	c.Set("Content-Type", "text/plain; charset=utf-8")
	c.Set("Cache-Control", "no-cache")

	return c.SendStreamWriter(func(w *bufio.Writer) {
		stream := a.Client.Models.GenerateContentStream(
			ctx,
			"gemini-3.1-flash-lite",
			genai.Text(userContent),
			config,
		)

		for resp, err := range stream {
			if err != nil {
				log.Printf("Streaming error: %v", err)
				return
			}

			if resp == nil {
				continue
			}

			text := resp.Text()

			if _, err := w.WriteString(text); err != nil {
				log.Printf("WRITE ERROR: %v", err)
				return
			}

			if err := w.Flush(); err != nil {
				log.Printf("FLUSH ERROR: %v", err)
				return
			}
		}
	})
}
