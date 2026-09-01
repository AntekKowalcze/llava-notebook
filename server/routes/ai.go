// Package routes provides the HTTP handler responsible for streaming AI
// responses into the application's Markdown note editor.
//
// Purpose:
//
// This package contains the AI request handler that validates incoming prompt
// data, combines the current document, text selection, and user instruction,
// sends the resulting context to the configured Google AI model, and streams
// generated Markdown content back to the client.
//
// The handler is designed for incremental response delivery so that the
// frontend can display generated content while the model is still producing
// the response.
//
// Exports:
//
//   - Ai — Validates an AI request, builds the model prompt, invokes the Google
//     AI streaming API, and writes generated text incrementally to the HTTP
//     response.
//
// Key design decisions:
//
// The model receives the document, selection, and instruction as explicitly
// labeled sections rather than as an unstructured prompt. This keeps the
// different parts of the editing context distinguishable and allows the
// system instruction to define how each part should be interpreted.
//
// A dedicated system instruction establishes the AI's role as a Markdown
// editor assistant and constrains the response to content that can be inserted
// directly into the editor. Explanations, wrapper formats, and Markdown code
// fences are explicitly disallowed by the prompt.
//
// AI output is streamed directly to the client using Fiber's stream writer.
// The server therefore does not need to buffer the complete generated
// response before sending it to the frontend.
//
// The response is sent as plain UTF-8 text with caching disabled because the
// client consumes the generated content as an incremental text stream rather
// than as a structured JSON response.
//
// Streaming errors are logged and terminate the stream rather than being
// serialized into the generated content. This keeps transport or model errors
// separate from the text consumed by the editor.
//
// The Google AI model is invoked with the request context obtained from Fiber,
// allowing request cancellation or termination to propagate to the model
// streaming operation.
//
// ## Dependencies
//
//   - github.com/gofiber/fiber/v3 — HTTP request handling and streaming
//     response support.
//   - google.golang.org/genai — Google AI client and streaming content
//     generation.
//   - llava-server/middleware — Standardized request validation error
//     responses.
//   - llava-server/models — Definition of the incoming AI request payload.
//   - bufio — Buffered HTTP stream writing.
//   - fmt — Construction of the structured user prompt.
//   - log — Logging of streaming and response-writing failures.
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
