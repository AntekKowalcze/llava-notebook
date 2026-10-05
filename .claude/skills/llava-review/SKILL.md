---
name: llava-review
description: Project-specific code review checklist for llava (Markdown notes app with local/cloud sync, Go server, Rust core, Vue/Tauri desktop). Use when reviewing a diff, PR or branch, or when asked "review my changes", alongside the generic /code-review.
---

# llava review checklist

Review `git diff` (or the given PR/branch). Report only issues you can point to in the code, with file:line and a concrete failure scenario. Order by severity.

## Server (Go, `server/`)
- Every route that touches user data sits behind auth middleware and scopes queries by the authenticated user ID, never by an ID from the request body alone.
- Errors are returned or logged, not swallowed; handlers return correct status codes and never leak internals or secrets.
- Passwords are hashed, tokens/secrets come from config or env, nothing is hard-coded or logged.
- Sync (`routes/sync.go`): check conflict handling, idempotency on retry, and deletion semantics so a stale client cannot overwrite newer notes.
- AI routes (`routes/ai.go`): validate input size, handle upstream failures and timeouts, don't pass unvalidated user input into URLs or shell.

## Core (Rust, `core/`)
- No `unwrap()`/`expect()` on paths driven by user data or I/O; propagate errors.
- Local storage writes are safe against partial failure (atomic write or transaction).
- Behaviour matches what the server expects in local↔cloud switching.

## Desktop (Vue/TS, `desktop/src`)
- Types are real (no new `any`); API response shapes match the server.
- Async calls have loading and error states; local and cloud modes both work.
- Editor (Milkdown) content is not lost on note switch, logout or mode change.
- Tauri commands and permissions are the minimum needed.

## Everywhere
- New behaviour has a test or a stated reason it doesn't.
- No leftover debug output, commented-out code, or unrelated edits in the diff.

Finish by running the `verify-changes` skill so the review includes real command results.
