---
name: verify-changes
description: Run the right build, lint, type-check and test commands for whatever part of the llava repo was changed (Go server, Rust core, Vue/Tauri desktop). Use this whenever code was edited and before saying work is done, committing, or opening a PR, even if the user just says "check it", "does it work" or "run tests".
---

# Verify changes

Pick checks from what actually changed, so verification is fast and nothing is skipped.

1. Run `git status --short` and `git diff --stat` to see which areas changed.
2. Run only the matching checks below. Fix failures at the root cause, then re-run.

| Changed path | Commands (run from that dir) |
|---|---|
| `server/**` (Go) | `cd server && gofmt -l . && go vet ./... && go build ./... && go test ./...` |
| `core/**` (Rust) | `cd core && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` |
| `desktop/src/**` (Vue/TS) | `cd desktop && npm run lint && npm run build && npm run test:ci` |
| `desktop/src-tauri/**` | `cd desktop/src-tauri && cargo check` then the desktop checks above |

Notes:
- `npm run build` includes `vue-tsc --noEmit`, so it doubles as the type check.
- If a change crosses areas (e.g. an API shape in `server/routes` and its caller in `desktop/src`), check both sides agree on field names and types. Compilers won't catch that.
- Report results plainly: which commands ran, which passed, which failed with the key error. If a check was skipped (missing toolchain, no tests), say so.
