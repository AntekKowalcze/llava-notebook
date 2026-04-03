pub mod auth_data_models;
pub mod database_creation;
pub mod logging;
pub mod register;
pub mod utils;
//TODO
//IN my head it looks like that
//1. one online account 1 user (for now) but multiple devices.
//2. Last write wins conflict (if its nice programming problem i will consider makign somethning better)
//3.

// Online Auth Foundation (now)
// Goal: prove online account <-> many local device accounts model.
// Implement:
// Mongo collections/schema for online_users, devices, sessions (or refresh tokens), optional workspaces.
// Tauri/core commands: online_register, online_login, link_local_device, unlink_device, get_link_status.
// Local DB mapping: keep is_online_linked, online_account_email, and add online_user_id if missing.
// Exit criteria:
// Same online account can be linked from 2 devices.
// Each device still has its own local user/profile and local keys.
// Login/logout works without touching note sync yet.
// Editor MVP (right after)
// Goal: establish real note CRUD user flow.
// Implement:
// Replace /main placeholder editor route with real note list + note edit + save.
// Ensure every create/update/delete updates sync_state correctly (PendingUpload where needed).
// Exit criteria:
// Fully usable local editor with durable saves.
// sync_state transitions happen consistently.
// Sync + Conflict MVP
// Goal: move data between local and online reliably.
// Implement:
// Upload/download pipeline, retry queue, per-note version tracking.
// Conflict detection + one deterministic policy first (even if basic).
// Exit criteria:
// Two devices can edit and converge with known conflict behavior.
// Failures are recoverable and visible in UI.
// Account Management + Mailer
// Goal: production-grade account lifecycle.
// Implement:
// Email verification/reset, device revocation UX, password/email change flows.
// Exit criteria:
// Recovery and account ops tested; no lockout edge cases.
// Where your assumption is strong

// “Editor is easier after auth decisions” is partly true: identity and linkage should come first.
// Where I push back

// Full “online account management” before editor is too broad.
// You need editor MVP early to validate sync semantics and conflict UX with real note operations.
// Data model warning
// For your “team-like shared online account”, decide now:

// Shared credentials (fast, risky) vs member identities (better long-term).
// If you choose shared credentials now, design schema so migration to members is possible later.
// Questions you should answer before coding Phase 1

// Is online account shared by whole team, or do you want per-person members under one workspace?
// Who can unlink/revoke devices?
// Do you need one workspace per online account, or multiple?
// What is the initial conflict rule (LWW/manual merge/duplicate-note)?
// Is sync opt-in per device or per note?
// If you want, I can draft a concrete Phase 1 implementation spec next:

// Mongo schema
// Rust structs/enums
// Tauri command signatures
// local DB migration changes
// test checklist for linking 2 devices to one online account.
