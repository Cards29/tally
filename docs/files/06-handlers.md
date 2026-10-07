# 06. Handlers

> Synced at commit `7e4f19b` · Prev: [05. Auth middleware](05-middleware-auth.md) · Next: [07. Errors](07-error.md) · [Index](../README.md)

Handlers are the functions the router calls. They are deliberately **thin**: take the input out of the request, call storage, and turn the result into an HTTP response. They contain no file logic and no business rules.

| File | Handlers |
|------|----------|
| `src/handlers/health.rs` | `check` |
| `src/handlers/log.rs` | `add_entry`, `show_log`, `show_last`, `clear_last`, `clear_all` |

No new concept docs here. These files put [axum](../concepts/web/axum.md) handlers, [pattern matching](../concepts/rust/pattern-matching.md), and [`?` with `From`](../concepts/rust/result-and-panics.md#the--operator) to work together.

---

## `src/handlers/health.rs`

```rust
use axum::http::StatusCode;

/// Liveness check. Always returns 200.
pub async fn check() -> StatusCode {
    StatusCode::OK
}
```

- It takes no extractors, because it needs nothing from the request.
- It returns `StatusCode`, which implements `IntoResponse`, so axum turns it into a `200 OK` response with an empty body.
- It's `async` even though it never awaits anything, because axum handlers must return a future.
- "Liveness" means "the process is up and answering". It doesn't touch storage on purpose: a broken disk shouldn't make the server look dead to monitors.

This is the function that `.cargo/mutants.toml` excludes one mutant of. Replacing the body with `Default::default()` still returns 200, since 200 is `StatusCode`'s default.

---

## `src/handlers/log.rs`

```rust
use axum::{extract::State, http::StatusCode};

use crate::{error::AppError, state::AppState, storage::log as store};

/// `POST /log`: appends the current time and returns the new entry.
pub async fn add_entry(State(state): State<AppState>) -> Result<String, AppError> {
    Ok(store::add_entry(&state.file_name)?)
}

/// `GET /log`: returns the whole log, one entry per line.
pub async fn show_log(State(state): State<AppState>) -> Result<String, AppError> {
    Ok(store::show_log(&state.file_name)?)
}

/// `GET /log/last`: returns the last entry, or an empty body if the log is empty.
pub async fn show_last(State(state): State<AppState>) -> Result<String, AppError> {
    Ok(store::show_last(&state.file_name)?)
}

/// `DELETE /log/last`: removes the last entry and returns 204.
pub async fn clear_last(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    store::clear_last(&state.file_name)?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /log`: empties the log and returns 204.
pub async fn clear_all(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    store::clear_all(&state.file_name)?;
    Ok(StatusCode::NO_CONTENT)
}
```

### Imports

```rust
use axum::{extract::State, http::StatusCode};

use crate::{error::AppError, state::AppState, storage::log as store};
```

`storage::log as store`: this file *is* the `handlers::log` module. Importing another module also named `log` would be confusing, so it's renamed to `store`. Now `store::add_entry` (storage) and this file's own `add_entry` (the handler) are clearly different things. The function names match on purpose: each handler calls the storage function with the same name.

### One handler, line by line

```rust
/// `POST /log`: appends the current time and returns the new entry.
pub async fn add_entry(State(state): State<AppState>) -> Result<String, AppError> {
```

- The doc comment starts with the route. When reading handlers, the first question is always "which URL calls this?".
- `State(state): State<AppState>`: extract the shared state and unwrap it with a pattern in the parameter.
- `-> Result<String, AppError>`:
  - `Ok(String)` becomes a **200** response with the string as a `text/plain` body.
  - `Err(AppError)` becomes a **500** response with an empty body, through `AppError`'s `IntoResponse` (doc 07).

```rust
    Ok(store::add_entry(&state.file_name)?)
}
```

This line packs in three things:

1. `&state.file_name`: borrow the `String` field. It becomes `&str` automatically, which is what storage takes.
2. `store::add_entry(...)` returns `anyhow::Result<String>`, which is `Result<String, anyhow::Error>`.
3. `?` unwraps the `String` on success. On error it **converts** the `anyhow::Error` into an `AppError` with `From::from`, and returns it. That conversion exists because of the blanket impl `impl<E: Into<anyhow::Error>> From<E> for AppError` in `error.rs`.
4. `Ok(...)` wraps the `String` again, now in `Result<String, AppError>`.

Why not just return `store::add_entry(&state.file_name)` directly? Its type is `Result<String, anyhow::Error>`, not `Result<String, AppError>`, and Rust doesn't convert return values automatically. Only `?` does a conversion. `Ok(x?)` is the usual idiom for "convert the error type, keep the value".

### The other handlers

`show_log` and `show_last` follow the same shape. Note that `show_last` on an empty log returns `Ok("")`, so the client gets **200 with an empty body**. Changing that to 404 is planned. The integration test `last_entry_on_empty_log_returns_ok_with_empty_body` pins the current behavior.

```rust
pub async fn clear_last(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    store::clear_last(&state.file_name)?;
    Ok(StatusCode::NO_CONTENT)
}
```

- The storage function returns `Result<()>`, so there's nothing to send back.
- `store::clear_last(...)?;`: the `;` throws away the `()`, and `?` still returns early on error.
- `Ok(StatusCode::NO_CONTENT)` is **204**: success, with no body by definition. That's the standard response for a DELETE that has nothing to return.

`clear_all` is the same, but calls `store::clear_all`.

---

## Why handlers stay thin

| Layer | Knows about |
|-------|-------------|
| Router (doc 04) | URLs, methods, which routes need auth |
| Middleware (doc 05) | Headers, the token |
| **Handlers** | HTTP status codes, response types |
| Storage (doc 08) | Files, lines, timestamps. Nothing about HTTP. |

Because storage knows nothing about HTTP, it can be unit-tested without a server. Swapping files for a database (the planned `LogStore`) will only touch storage and state; the handlers won't need to change much.

---

## Try it

1. Change `add_entry`'s body to `store::add_entry(&state.file_name)` (without `Ok(...?)`) and read the type error.
2. Make `clear_all` return `Ok(StatusCode::OK)` instead of 204, then run `cargo test`. Which test catches it? Then change it back.
3. Remove `Ok(...)` and the `?`, and write `store::add_entry(&state.file_name).map_err(AppError::from)`. Does it compile? It does the same thing. The repo uses `Ok(x?)`.

## Quick revise

- Handlers are thin: extract, call storage, map the result to HTTP. No file logic.
- `health::check` returns `StatusCode::OK`, and doesn't touch storage.
- `storage::log as store` avoids the `log`/`log` name clash. Handler names match storage function names.
- `Result<String, AppError>`: `Ok` gives 200 + text, `Err` gives 500 + an empty body.
- `Ok(store::f(&state.file_name)?)`: `?` converts `anyhow::Error` into `AppError` through `From`. Return values are not converted automatically.
- DELETE routes return `StatusCode::NO_CONTENT` (204).
- `GET /log/last` on an empty log currently returns 200 + an empty body. 404 is planned.
