# rust_web_app

Rust learning project: an axum + tokio web app that logs button-press timestamps sent from the owner's phone. Entries belong to a user and live in Postgres. Deployed on Render (free tier) from git.

## Working rules

- **Suggest code only.** Do not edit or create source files unless the user explicitly says "write the code". The user is learning Rust and types the code themselves.
- Work in checkpoints: show the full proposed change, then stop and wait for approval.
- Explain new Rust concepts briefly when they appear. Keep explanations tight.
- Keep changes minimal and targeted. No new features, refactors, or abstractions unless asked.
- Branch: work only on `dev`. Never push, merge, or touch `main`.
- Ask before: adding any dependency, deleting any file, or running any git command other than `git status` / `git diff`.
- Never read or print `.env` or `dates.log`. Use obviously fake values in tests (e.g. `"tly_wrong"`).

## Layout

- `src/main.rs`: clap CLI. `Config::from_env()?`, then one of:
  - `serve` (default): runs `sqlx::migrate!()`, then `routes::router(config.state)`, binds `0.0.0.0:{port}` and serves. Migrations run only here, so the CLI pointed at Neon never applies dev migrations to prod.
  - `create-admin --handle <h> --display-name <n>`: inserts an admin user.
  - `create-device-token --handle <h> --name <n>`: prints a new device token once.
- `src/config.rs`: `Config { state: AppState, port }`. `from_env` loads `.env` via `dotenvy`, then reads `DATABASE_URL` (required; `PgPool::connect_lazy`, so it stays sync), `STORAGE` (`postgres` default, or `file`), `FILE_NAME` (required only when `STORAGE=file`) and `PORT` (default 3000). See `.env.example`.
- `src/state.rs`: `AppState { pool: PgPool, log: LogStore }`, `Clone`. Users and sessions always live in `pool`, whatever `log` is.
- `src/error.rs`: `AppError(anyhow::Error)` newtype. `IntoResponse` logs the error and returns a bare 500. Blanket `From<E: Into<anyhow::Error>>`.
- `src/routes.rs`: builds the router.
- `src/handlers/{health,log}.rs`, `src/middleware/auth.rs`, `src/storage/{log,log_store,postgres,tokens,users}.rs`.
- Parent module files (`handlers.rs`, `middleware.rs`, `storage.rs`) only hold `pub mod` lines.
- `migrations/0001_init.sql`: `users`, `sessions`, `entries`. Every `id` is `uuid default uuidv7()` (built into Postgres 18), so inserts omit it. `handle` is `citext`.
- `build.rs`: `rerun-if-changed=migrations`, so `sqlx::migrate!()` picks up new migrations.
- `.sqlx/`: committed offline query data, so builds work without a database. After changing any `query!`, run `cargo sqlx prepare -- --all-targets` and commit `.sqlx/`.

## Routes

- Public: `GET /` (temporary redirect to `/health`), `GET /health` (200).
- Protected by `route_layer(from_fn_with_state(state.clone(), auth::require_token))`. Each acts only on the caller's own entries:
  - `POST /log` adds an entry and returns it
  - `GET /log` returns the whole log, one entry per line
  - `DELETE /log` clears the log (204)
  - `GET /log/last` returns the last entry
  - `DELETE /log/last` removes the last entry (204)
- Entries are formatted in UTC: `%a, %b %d %Y %H:%M:%S UTC` (`TIME_FORMAT` in `handlers/log.rs`).
- Auth: `Authorization: Bearer <token>`. The SHA-256 of the token is looked up in `sessions` (unexpired only). 401 if the header is missing or no session matches. On success, `CurrentUser { id }` goes into request extensions and handlers read it with `Extension<CurrentUser>`. `sessions.last_used_at` and `users.last_seen_at` are updated at most once an hour per session.

## Storage

Entries are `DateTime<Utc>`. `LogStore` (`storage/log_store.rs`) is an enum, `File(String)` or `Postgres(PgPool)`, picked by `STORAGE`. It is an enum, not a trait, because async trait methods can't be `dyn`, and a generic `AppState<S>` would spread a type parameter through every route. Its async methods take `user_id`, `match` on the store and delegate. Function names match the handlers:
- `add_entry` returns the new entry, `show_log` returns all entries oldest first, `show_last` returns `Option`, and `clear_last` and `clear_all` return `()`.

`storage/postgres.rs`: free functions taking `(&PgPool, Uuid)`, using `query!` / `query_scalar!`. Ordering is `created_at, id`. `clear_last` is one `DELETE ... WHERE id = (SELECT ... LIMIT 1)`, so it is atomic.

`storage/log.rs`: the file store, using blocking `std::fs`. Single-user: ignores `user_id`, so every user shares one log. It writes one RFC 3339 UTC line per entry (`to_rfc3339_opts(AutoSi, true)`). A missing file (`NotFound`) means an empty log, not an error. `show_last` and `clear_last` go through `show_log`.

`storage/tokens.rs`: `generate()` returns `tly_` plus base64url (no padding) of 32 bytes from `rand::rngs::SysRng`. `hash()` returns the token's SHA-256. Only hashes are stored.

`storage/users.rs`: `create_admin` and `create_device_token` (`insert ... select ... where handle = $1`; bails `no user with handle {handle}` if no row was inserted).

Known current behavior: `GET /log/last` on an empty log returns 200 with an empty body. Step 2 changes this to 404.

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

The cloud-storage round is planned step by step outside this file (step 1 done: Postgres, users, device tokens, CLI). Next:
- Step 2: JSON responses, `AppError` becomes an enum (400/401/403/404/...), `GET /log/last` returns 404 on an empty log.
- Step 3: deploy on Neon (`SQLX_OFFLINE=true` on Render).
- Later steps: accounts, invites, email, hardening.
- Timezones: entries are stored and shown in UTC. Per-user zones (`users.timezone`, `chrono-tz`, an `X-Timezone` header) come later. Don't set `TZ` on Render: nothing reads local time any more.
- The file store's `clear_last` is still a non-atomic read-then-write (the Postgres store's isn't).
- Store the file log path as `PathBuf` / `&Path` instead of `String` / `&str`. Removes the `.to_str().expect(...)` in both `temp_log()` helpers. Cost: error messages need `file_name.display()`.
- `uuid` crate's `v7` feature is unused (the DB generates ids). Drop it if no Rust code ever calls `Uuid::now_v7()`.

## Docs

- `docs/` is a Rust/backend learning guide for the user, written *after* the code. It describes the code; it never drives it.
- Do not use `docs/` as a source for design, plans, or conventions. Read the code and this file instead.
- Claude writes and commits `docs/` (an exception to "suggest code only"), but only when the user asks, after a big coding part is finished.
- Layout: `docs/README.md` (index + reading paths), `docs/files/NN-*.md` (line-by-line per file group, with `Synced at commit <hash>`), `docs/concepts/{rust,web,tooling}/*.md` (one idea each, written when first seen).
- Every doc starts with YAML frontmatter (`tags`, `aliases`) and ends with "Try it" and a "Quick revise" section in a `> [!TIP]` callout. Docs are plain English, not caveman.
- Docs are read in Obsidian and on GitHub: use relative `[text](path.md)` links only. No `[[wikilinks]]`, no `#heading` anchors (they differ between the two). Callouts only `NOTE`/`TIP`/`IMPORTANT`/`WARNING`/`CAUTION`.
- To update: `git diff <synced hash>`, update affected file docs, add new concept docs, bump hashes, update README links.

## Tests

Tests assert current behavior. Never change app behavior to make a test pass. 30 tests (22 unit + 8 integration), not full coverage.

- DB tests use `#[sqlx::test]`: each test gets a fresh temporary database with migrations applied, dropped afterward. They need the local Postgres container running and `DATABASE_URL` set (from `.env`).
- `src/lib.rs` holds all `pub mod` lines so `tests/` can import `tally::...`. Dev-dependencies: `tempfile` (auto-deleted temp dirs) and `tower` with `util` (`ServiceExt::oneshot`).
- Unit tests: `#[cfg(test)] mod tests` at the bottom of `storage/{log,postgres,tokens,users}.rs`. `log.rs` tests share a `temp_log()` helper; `postgres.rs` tests insert users with a `user()` helper. `use super::*` already brings in the parent's imports.
- Integration tests: `tests/routes.rs` drives `routes::router(state)` with `oneshot`. Helpers: `PROTECTED` (all protected method+path pairs), `temp_log()`, `app(&pool)` (Postgres store), `user_token(&pool, handle)` (creates a user through `users::create_admin` + `users::create_device_token` and returns the token), and `send(&app, method, uri, token) -> (StatusCode, String)`, which builds the request and reads the body. Every test goes through `send`.
- To force a storage error, use the file store and create a directory at the log path (reading a directory as a file fails).
- `last_entry_on_empty_log_returns_ok_with_empty_body` asserts 200 + empty body. Update it when the 404 change lands.
- Mutation testing: `cargo mutants`, configured in `.cargo/mutants.toml` (skips `main.rs` and equivalent mutants in `health.rs` and `auth.rs`). Needs the local DB. `/tmp` is a 7.7G tmpfs: use `-j 2`, or set `TMPDIR` to a directory on disk. If most mutants come back unviable, check the logs for `Disk quota exceeded` before trusting `missed.txt`. `mutants.out*` is gitignored.

Test style:
- Behavior names without a `test_` prefix.
- Tests return `()` and use `expect("... should ...")`, never `unwrap`.
- Arrange, act and assert separated by blank lines. Reading a file back belongs to the assert block. Multi-step tests group each step under a comment.
- `assert_eq!(actual, expected)`. Always assert the status, not just the body: error responses have an empty body too.
- Discard unused values with `_`. Reuse names by shadowing (`let (status, log) = ...` again) instead of numbering them.
- Seed files with known text. Never assert exact timestamps; pin the format string in the test instead of reusing the constant.
- No extra test crates.
