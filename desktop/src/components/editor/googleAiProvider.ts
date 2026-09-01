/**
 * # AI provider module
 *
 * **Purpose**: This module provides the frontend abstraction used to stream
 * AI-generated responses into the application.
 *
 * It defines a common provider interface based on `AsyncIterable<string>` and
 * implements the Google AI provider using a Tauri channel for incremental
 * response delivery.
 *
 * ## Exports
 *
 * * [`AIProvider`] — Common interface for an AI provider that accepts prompt
 *   context and an `AbortSignal` and asynchronously yields response chunks.
 * * [`createGoogleProvider`] — Creates the Google AI provider which invokes
 *   the Tauri backend command and exposes the streamed response as an
 *   `AsyncIterable`.
 *
 * ## Key design decisions
 *
 * AI responses are exposed as an `AsyncIterable<string>` so consumers can
 * process generated content incrementally instead of waiting for the complete
 * response.
 *
 * The provider uses a Tauri [`Channel`] to receive response chunks from the
 * Rust backend. Incoming chunks are temporarily stored in an in-memory queue
 * and consumed by the async generator in arrival order.
 *
 * A promise-based wake mechanism is used to suspend the generator while the
 * queue is empty without continuously polling for new data.
 *
 * [`AbortSignal`] is integrated directly into the streaming loop. Aborting the
 * request wakes the pending generator immediately, allowing the provider to
 * terminate even when no new response chunk arrives.
 *
 * The underlying Tauri invocation is always awaited in the generator's
 * `finally` block. This ensures the backend request has completed before the
 * provider finishes cleanup, including when the consumer stops iteration
 * early.
 *
 * Backend request failures are converted into a user-visible toast instead of
 * being propagated as a provider exception. A successful request completes
 * the stream normally after all queued chunks have been consumed.
 *
 * ## Dependencies
 *
 * * `@tauri-apps/api/core` — Provides Tauri `invoke` and [`Channel`] for
 *   communication with the Rust backend.
 * * `vue-toastification` — Displays a warning when the AI request fails.
 * * `AbortSignal` — Provides cancellation support for the streaming request.
 */
import { Channel, invoke } from "@tauri-apps/api/core"
import { useToast } from "vue-toastification"
interface AIPromptContext {
  document: string
  selection: string
  instruction: string
}

export type AIProvider = (
  context: AIPromptContext,
  signal: AbortSignal,
) => AsyncIterable<string>

export function createGoogleProvider(): AIProvider {
  return async function* googleProvider(
    context: AIPromptContext,
    signal: AbortSignal,
  ) {
    const channel = new Channel<string>()
    const toast = useToast()

    const queue: string[] = []
    let resolveNext: (() => void) | null = null
    let finished = false
    let requestFailed = false

    const wake = () => {
      resolveNext?.()
      resolveNext = null
    }

    channel.onmessage = (chunk) => {
      queue.push(chunk)
      wake()
    }

    // Without this, aborting mid-wait didn't wake the loop — it just
    // sat there until the next chunk arrived or the stream finished.
    const onAbort = () => wake()
    signal.addEventListener("abort", onAbort)

    const request = invoke("ai_request", {
      channel,
      promptContext: context,
    }).then(() => {
      finished = true
      wake()
    }).catch(() => {
      finished = true
      requestFailed = true
      wake()
    })

    try {
      while (!signal.aborted) {
        if (queue.length > 0) {
          yield queue.shift()!
          continue
        }

        if (finished) {
          if (requestFailed) {
            toast.warning("Failed to get response from AI model")
          }
          return
        }

        await new Promise<void>((resolve) => {
          resolveNext = resolve
        })
      }
    } finally {
      signal.removeEventListener("abort", onAbort)
      await request
    }
  }
}