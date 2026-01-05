# Copilot / AI agent instructions for jellyfin-rpc

Short, actionable guidance for code completion and automated edits.

## What this project is
- Small Rust service/CLI that polls a Jellyfin server and updates Discord Rich Presence.
- Major components: `src/main.rs` (orchestrator + state), `src/jellyfin.rs` (Jellyfin API models + session selection), `src/discord.rs` (Discord IPC payloads), `src/covers.rs` (cover image URL builder), and `src/env_utils.rs` (.env loader).

## Big-picture architecture
- The program polls `GET {JELLYFIN_URL}/Sessions` and selects a session for `JELLYFIN_USER` using `pick_session_for_user` in `src/jellyfin.rs`.
- `main.rs` maintains an in-memory `State` struct tracking per-session last positions, last items, null-gap timers, and whether a rewind adjustment has been applied.
- When a session is chosen, `src/discord.rs::set_activity` builds a Discord IPC JSON payload and sends it via `discord_rich_presence` IPC.
- Cover images are constructed by `src/covers.rs::get_cover_url` hitting `{JELLYFIN_URL}/Items/<id>/Images/Primary?maxWidth=512&quality=90&api_key=<API_KEY>`.

## Key files to reference when making edits
- Polling / orchestration: [src/main.rs](src/main.rs)
- Jellyfin data models and session selection: [src/jellyfin.rs](src/jellyfin.rs)
- Discord payload & activity formatting: [src/discord.rs](src/discord.rs)
- Cover URL construction: [src/covers.rs](src/covers.rs)
- .env loading behavior: [src/env_utils.rs](src/env_utils.rs)

## Environment variables and runtime notes
- Required env vars (see `main.rs` statics): `DISCORD_CLIENT_ID`, `JELLYFIN_URL`, `JELLYFIN_API_KEY`, `JELLYFIN_USER`.
- Tuning vars: `JELLYFIN_POLL_INTERVAL_SECS`, `DISCORD_UPDATE_INTERVAL_SECS`, `NULL_GAP_REWIND_SECS`, `NULL_GAP_MAX_SECS` (all parsed as integers).
- `.env` loading: `load_local_env()` looks for a `.env` file next to the compiled executable (exe directory). When running with `cargo run` the executable lives under `target/debug/` — put `.env` there or export vars in your shell.

## Conventions and patterns to follow
- Use `serde::Deserialize` with field renames matching the Jellyfin API (see `src/jellyfin.rs`) — do not change field names unless Jellyfin responses change.
- Session selection rules live in `pick_session_for_user`: prefer sessions that belong to `JELLYFIN_USER` (case-insensitive), prefer a session with a real NowPlaying item over one that does not, and prefer larger `position_ticks` when tied. Keep changes consistent with this priority logic.
- `State` in `main.rs` is authoritative for tracked session state; update it consistently when mutating last item/pos/null timers.
- Discord activity timestamps are computed relative to current system time and clamped against runtime; preserve the existing timestamp logic in `src/discord.rs` when editing presence behavior.

## Build / run / debug commands (practical)
- Build: `cargo build` or `cargo build --release`
- Run locally (recommended):
  - ensure env vars are exported, or place a `.env` next to the executable (`target/debug/` when using `cargo run`)
  - `cargo run --release` (or `cargo run` for debug)
- Tests: `cargo test` (project currently has no tests in tree; run to verify no regressions)

## Integration points / external dependencies
- Contacts external services: Jellyfin (HTTP JSON), Discord (IPC via `discord_rich_presence`). When editing network logic consider timeouts and error handling (client is `reqwest::blocking::Client`).
- `src/covers.rs` includes the API key in query params — changing this affects image URLs returned to Discord.

## Examples of common changes and where to apply them
- Add new per-session metric: extend `State` in `src/main.rs` and persist update logic near where `last_item_by_session`/`last_pos_by_session` are updated.
- Change selection heuristics: edit `pick_session_for_user` in `src/jellyfin.rs` (tests or manual runs recommended).
- Change activity formatting: edit `set_activity` in `src/discord.rs` — for types `Audio`, `Movie`, `Episode` the code maps fields explicitly; follow the same pattern for new types.

## Safety and backward-compat checks for PRs
- Run `cargo build` after changes. Manual run with real Jellyfin/Discord or mocked endpoints is the fastest verification.
- Avoid changing `serde` renames without confirming Jellyfin API responses.

If anything here is unclear or you'd like more examples (e.g., sample `.env` or a small unit test for `pick_session_for_user`), tell me which area to expand.
