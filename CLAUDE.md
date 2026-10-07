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

- `src/main.rs`: `Config::from_env()?` → `routes::router(config.state)` → bind `0.0.0.0:{port}` + serve.
- `src/config.rs`: `Config { state: AppState, port }`. `from_env` loads `.env` via `dotenvy`, then reads `FILE_NAME`, `AUTH_TOKEN`, `PORT` (default 3000).
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

`src/storage/log.rs` uses blocking `std::fs`. Function names match the handlers in `src/handlers/log.rs`:
- `add_entry`: appends the current local time (`TIME_FORMAT` = `%a, %b %d %Y %H:%M:%S`, 24-hour) and returns it.
- `show_log`: returns the file contents. A missing file (`NotFound`) means an empty log, not an error.
- `show_last` and `clear_last` both use the private `split_last_line`, so they agree on what "last line" means.
- `clear_all`.

Known current behavior: `GET /log/last` on an empty log returns 200 with an empty body. This is planned to become 404 later.

## Conventions

- Errors: use `?` with anyhow `with_context`, never `.context`. Error messages are lowercase and name the file when there is one (`format!("failed to read {file_name}")`).
- Format strings use inline args: `"{file_name}"`, not `"{}", file_name`.
- Imports grouped std → external crates → `crate::`, alphabetized and merged per crate (`use std::{fs::..., io::...};`).
- `mod` declarations alphabetized.
- Lints: lint levels live in `Cargo.toml` `[lints.clippy]` (`unwrap_used = "warn"`). Lint settings live in `clippy.toml` (`disallowed-methods` bans `anyhow::Context::context`).
- Before finishing: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` must all be clean. After changing tests or storage logic, also run `cargo mutants`: `mutants.out/missed.txt` must be empty.

## Versioning

- `main` holds released versions. `dev` holds the next version.
- After each merge of `dev` into `main`, bump `dev`'s patch version by one (0.1.1 → 0.1.2).
- A proper release is `1.0.0`. After it is merged to `main`, `dev` starts at `1.0.1`.
- Never change the version on `main` directly. It only changes through a merge from `dev`.
- Version lives only in `Cargo.toml` `[package].version`.

## Planned (not yet done)

- `LogStore` trait for storage (do not add until asked).
- `GET /log/last` returns 404 on an empty log.
- Move storage to a cloud database. Render's free-tier filesystem is wiped on spin-down and deploy.
- Timezones: Render runs in UTC, so `chrono::Local` stamps entries in UTC, not the owner's time.
  - Now (no code): set `TZ` in Render's environment (e.g. `TZ=Asia/Dhaka`). If it still shows UTC, the image lacks tzdata; use a POSIX string instead (`TZ=<+06>-6` for UTC+6, sign inverted). Zone is fixed, so travel isn't handled.
  - With the cloud database: store UTC instants (`DateTime<Utc>` / Postgres `timestamptz`), not formatted strings. The client sends its zone (e.g. an `X-Timezone: Asia/Dhaka` header) and the server formats entries in that zone on read. Needs the `chrono-tz` crate (ask before adding).
- `clear_last` is a non-atomic read-then-write: a `POST /log` between the read and the write is lost, and a crash mid-write truncates the log. Fix when `LogStore` lands (e.g. a mutex in the store).
- Store the log path as `PathBuf` / `&Path` instead of `String` / `&str`. Removes the `.to_str().expect(...)` in both `temp_log()` helpers. Cost: error messages need `file_name.display()`.

## Docs

- `docs/` is a Rust/backend learning guide for the user, written *after* the code. It describes the code; it never drives it.
- Do not use `docs/` as a source for design, plans, or conventions. Read the code and this file instead.
- Claude writes and commits `docs/` (an exception to "suggest code only"), but only when the user asks, after a big coding part is finished.
- Layout: `docs/README.md` (index + reading paths), `docs/files/NN-*.md` (line-by-line per file group, with `Synced at commit <hash>`), `docs/concepts/{rust,web,tooling}/*.md` (one idea each, written when first seen).
- Every doc ends with "Try it" and "Quick revise". Docs are plain English, not caveman.
- To update: `git diff <synced hash>`, update affected file docs, add new concept docs, bump hashes, update README links.

## Tests

Tests assert current behavior. Never change app behavior to make a test pass. 15 tests (9 unit + 6 integration), not full coverage.

- `src/lib.rs` holds all `pub mod` lines so `tests/` can import `tally::...`. Dev-dependencies: `tempfile` (auto-deleted temp dirs) and `tower` with `util` (`ServiceExt::oneshot`).
- Unit tests: `#[cfg(test)] mod tests` at the bottom of `src/storage/log.rs`, sharing a `temp_log()` helper. `use super::*` already brings in the parent's imports (e.g. `fs`).
- Integration tests: `tests/routes.rs` drives `routes::router(state)` with `oneshot`. Helpers: `TOKEN = "test-token"`, `PROTECTED` (all protected method+path pairs), `temp_log()`, `app(file_name)`, and `send(&app, method, uri, token) -> (StatusCode, String)`, which builds the request and reads the body. Every test goes through `send`.
- Every test uses `temp_log()`, even ones that never reach storage, so a broken auth layer can't write a real file.
- To force a storage error, create a directory at the log path (reading a directory as a file fails).
- `last_entry_on_empty_log_returns_ok_with_empty_body` asserts 200 + empty body. Update it when the 404 change lands.
- Mutation testing: `cargo mutants`, configured in `.cargo/mutants.toml` (skips `main.rs` and one equivalent mutant in `health.rs`). `mutants.out*` is gitignored.

Test style:
- Behavior names without a `test_` prefix.
- Tests return `()` and use `expect("... should ...")`, never `unwrap`.
- Arrange, act and assert separated by blank lines. Reading a file back belongs to the assert block. Multi-step tests group each step under a comment.
- `assert_eq!(actual, expected)`. Always assert the status, not just the body: error responses have an empty body too.
- Discard unused values with `_`. Reuse names by shadowing (`let (status, log) = ...` again) instead of numbering them.
- Seed files with known text. Never assert exact timestamps; pin the format string in the test instead of reusing the constant.
- No extra test crates.
