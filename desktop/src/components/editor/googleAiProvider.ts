import { Channel, invoke } from "@tauri-apps/api/core"
import { useToast } from "vue-toastification"
import type { ToastInterface } from "vue-toastification"
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