/**
 * Errors from Tauri commands are the serialised Rust `Error` enum: a unit
 * variant arrives as a plain string (`"SyncFailed"`), a tuple variant as a
 * single-key object (`{ AccountLocked: 5 }`). Anything else (a Tauri argument
 * error, a thrown `Error`) has no variant.
 */

/** Name of the Rust `Error` variant, or `''` when `err` is not one. */
export function errorKey(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err && typeof err === 'object' && !(err instanceof Error)) {
    const keys = Object.keys(err);
    if (keys.length === 1) return keys[0];
  }
  return '';
}

/** True when `err` is the given Rust `Error` variant. */
export function isError(err: unknown, variant: string): boolean {
  return errorKey(err) === variant;
}

/** Payload of a tuple variant (`{ AccountLocked: 5 }` -> `5`). */
export function errorPayload<T = unknown>(err: unknown): T | undefined {
  if (err && typeof err === 'object' && !(err instanceof Error)) {
    return Object.values(err)[0] as T;
  }
  return undefined;
}
