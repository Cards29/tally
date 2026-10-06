# rust_web_app

Rust learning project: an axum + tokio web app that logs button-press timestamps sent from the owner's phone. Deployed on Render (free tier) from git.

## Working rules

- **Suggest code only.** Do not edit or create source files unless the user explicitly says "write the code". The user is learning Rust and types the code themselves.
- Work in checkpoints: show the full proposed change, then stop and wait for approval.
- Explain new Rust concepts briefly when they appear. Keep explanations tight.
- Keep changes minimal and targeted. No new features, refactors, or abstractions unless asked.
- Branch: work only on `dev`. Never push, merge, or touch `main`.
- Ask before: adding any dependency, deleting any file, or running any git command other than `git status` / `git diff`.
- Never read or print `.env` or `dates.log`. Use fake values like `"test-token"` in tests.

## Layout

- `src/main.rs`: `Config::from_env()?` → `AppState { file_name, auth_token }` → `routes::router(state)` → bind `0.0.0.0:{port}` + serve.
- `src/config.rs`: reads `FILE_NAME`, `AUTH_TOKEN`, `PORT` (default 3000). `dotenvy` loads `.env`.
- `src/state.rs`: `AppState { file_name, auth_token }`, `Clone`.
- `src/error.rs`: `AppError(anyhow::Error)` newtype. `IntoResponse` logs the error and returns a bare 500. Blanket `From<E: Into<anyhow::Error>>`.
- `src/routes.rs`: builds the router.
- `src/handlers/{health,log}.rs`, `src/middleware/auth.rs`, `src/storage/log.rs`.
- Parent module files (`handlers.rs`, `middleware.rs`, `storage.rs`) only hold `pub mod` lines.

## Routes

- Public: `GET /` (temporary redirect to `/health`), `GET /health` (200).
- Protected by `route_layer(from_fn_with_state(state.clone(), auth::require_token))`:
  - `POST /log` adds an entry and returns it
  - `GET /log` returns the whole log
  - `DELETE /log` clears the log (204)
  - `GET /log/last` returns the last entry
  - `DELETE /log/last` removes the last entry (204)
- Auth: `Authorization: Bearer <AUTH_TOKEN>`, constant-time compare via `subtle`, 401 if missing or wrong.

## Storage

`src/storage/log.rs` uses blocking `std::fs`:
- `log_time`: appends the current local time (`%a, %b %d %Y %H:%M:%S`) and returns it.
- `read_log`: returns the file contents. A missing file (`NotFound`) means an empty log, not an error.
- `clear_log`, `clear_last_entry`, `last_entry`.

Known current behavior: `GET /log/last` on an empty log returns 200 with an empty body. This is planned to become 404 later.

## Conventions

- Errors: use `?` with anyhow `with_context`. Error messages are lowercase.
- Imports grouped std → external crates → `crate::`, alphabetized and merged per crate.
- `mod` declarations alphabetized.
- Before finishing: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` must all be clean.

## Versioning

- `main` holds released versions. `dev` holds the next version.
- After each merge of `dev` into `main`, bump `dev`'s patch version by one (0.1.1 → 0.1.2).
- A proper release is `1.0.0`. After it is merged to `main`, `dev` starts at `1.0.1`.
- Never change the version on `main` directly. It only changes through a merge from `dev`.
- Version lives only in `Cargo.toml` `[package].version`.

## Planned (not yet done)

- `LogStore` trait for storage (do not add until asked).
- `GET /log/last` returns 404 on an empty log.

## In progress: first test suite

Tests must assert current behavior. Do not change app behavior. 12 tests total (7 unit + 5 integration), not full coverage.

1. **Done.** `src/lib.rs` holds all `pub mod` lines; `src/main.rs` imports `tally::...`. Dev-dependencies added: `tempfile` (auto-deleted temp dirs per test) and `tower` with `util` (`ServiceExt::oneshot`). The unused `delete` import in `routes.rs` was removed.
2. **Done.** Storage unit tests (7) in a `#[cfg(test)] mod tests` block at the bottom of `src/storage/log.rs`, sharing a `temp_log()` helper. Redundant extras were removed. `UNIT_TESTS.md` was deleted. `Cargo.toml` sets `[lints.clippy] unwrap_used = "warn"`.
3. **In progress: integration tests (5)** in `tests/routes.rs`, using `routes::router(state)` with `oneshot`. `INTEGRATION_TESTS.md` (repo root) holds the full verified file for the user to type from; delete it once the tests are typed and pass.
   - Helpers: `const TOKEN: &str = "test-token"`, `const PROTECTED: [(Method, &str); 5]` (all protected method+path pairs), `fn temp_log() -> Result<(TempDir, String)>`, `fn app(file_name) -> Router`, `fn request(method, uri, token: Option<&str>)`, `async fn body_text(response)` using `axum::body::to_bytes`.
   - Every test uses `temp_log()`, even ones that never reach storage, so a broken auth layer can't write a real file.
   - `protected_routes_reject_missing_token`: loop over `PROTECTED`, each 401, with `"{method} {uri}"` in the assert message.
   - `protected_routes_reject_wrong_token`: same loop with `Bearer wrong-token`.
   - `log_lifecycle_with_valid_token`: POST ×2 (save bodies as `first`/`second`) → GET /log == `"{first}\n{second}\n"` → GET /log/last == `second` → DELETE /log/last (204) → GET /log == `"{first}\n"` → DELETE /log (204) → GET /log (empty).
   - `last_entry_on_empty_log_returns_ok_with_empty_body`: 200 + empty body, with a comment that it will become 404.
   - `health_returns_ok_without_token`: 200 with no token.

Test style: behavior names without a `test_` prefix, tests return `anyhow::Result<()>` and use `?`, arrange/act/assert separated by blank lines, `assert_eq!(actual, expected)`, seed files with known text, never assert exact timestamps, no extra test crates.
