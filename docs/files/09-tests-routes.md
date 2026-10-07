# 09. Integration tests

> Synced at commit `7e4f19b` · Prev: [08. Storage](08-storage-log.md) · [Index](../README.md)

`tests/routes.rs` tests the **whole app** through the router: auth, routing, handlers, storage and errors together. It doesn't open a network port. Requests go straight into the `Router` as function calls.

Background: [Testing](../concepts/rust/testing.md), [axum](../concepts/web/axum.md#testing-without-a-network), [HTTP basics](../concepts/web/http-basics.md).

---

## Imports

```rust
use std::fs;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use tempfile::TempDir;
use tower::ServiceExt;

use tally::{routes, state::AppState};
```

- `Body`: an HTTP body type. `Body::empty()` makes an empty one.
- `to_bytes`: reads a whole body into memory.
- `Method`: `GET`, `POST`, `DELETE`, ...
- `Request`: here, `http::Request`, which has a `builder()`.
- `TempDir`: from the dev-dependency `tempfile`.
- `tower::ServiceExt`: an **extension trait**. Importing it adds `.oneshot()` to `Router`. This is why `tower` (with the `util` feature) is a dev-dependency.
- `tally::...`: the app is imported **as a library**, like an outside user would. That's only possible because of `src/lib.rs`.

---

## Constants

```rust
const TOKEN: &str = "test-token";
```

A fake token. Tests never use the real secret.

```rust
const PROTECTED: [(Method, &str); 5] = [
    (Method::POST, "/log"),
    (Method::GET, "/log"),
    (Method::DELETE, "/log"),
    (Method::GET, "/log/last"),
    (Method::DELETE, "/log/last"),
];
```

- `[(Method, &str); 5]` is an **array** type: a fixed length of 5, where each element is a **tuple** of `(Method, &str)`.
- `Method::POST` and the others are associated constants, so they can be used inside a `const`.
- Listing every protected route in one place means the auth tests loop over all of them. Adding a route means adding one line here.

---

## Helpers

### `temp_log`

```rust
#[track_caller]
fn temp_log() -> (TempDir, String) { ... }
```

The same helper as in the storage unit tests (see [doc 08](08-storage-log.md#the-helper)). Integration tests are a separate crate, so they can't reuse the private one from `src/`. It's duplicated on purpose.

### `app`

```rust
fn app(file_name: &str) -> Router {
    routes::router(AppState {
        file_name: file_name.to_string(),
        auth_token: TOKEN.to_string(),
    })
}
```

Builds the real router with test state. There's no `.env` and no `Config::from_env()`: tests never depend on environment variables. `.to_string()` turns each `&str` into the owned `String` the struct needs.

### `send`

```rust
async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
) -> (StatusCode, String) {
```

Every test goes through this function. It takes a borrowed router, the method, the path, and an **optional** token (`None` means no `Authorization` header). It returns the two things every test checks: the status and the body as text.

```rust
    let mut request = Request::builder().method(method).uri(uri);
```

The start of a request **builder**. It's `mut` because the next lines may add to it.

```rust
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
```

- `if let Some(token) = token`: only when a token was given. Inside the block, `token` **shadows** the `Option` and is a plain `&str`.
- `.header(...)` consumes the builder and returns it with the header added, so the result is assigned back to `request`.

```rust
    let request = request
        .body(Body::empty())
        .expect("request should be buildable");
```

`.body(...)` finishes the builder and gives `Result<Request<Body>, http::Error>`. It can fail on bad input, like an invalid URI, hence the `expect`. Shadowing reuses the name: `request` was the builder and is now the finished request.

```rust
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("router should respond");
```

- `.oneshot(request)` (from `ServiceExt`) sends one request through the router as a tower `Service` and gives a future of the response. No socket, no HTTP client.
- `oneshot` takes the service **by value** (it consumes it), and we only have `&Router`. So `.clone()` it first. Cloning a `Router` is cheap: it's built so that copies share the same routes internally.
- `.await`, then `.expect(...)`: a `Router`'s error type is `Infallible` (it can never fail), but the type is still a `Result`, so it has to be unwrapped.

```rust
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body = String::from_utf8(bytes.to_vec()).expect("response body should be utf-8");

    (status, body)
}
```

- `response.status()`: a `StatusCode`, which is `Copy`.
- `response.into_body()` **consumes** the response (the `into_` prefix) and gives its `Body`. The status was saved first because of this.
- `to_bytes(body, limit)` collects a streamed body into one `Bytes` value. `usize::MAX` means "no size limit". That's fine in tests; on a server you'd always set one.
- `String::from_utf8(Vec<u8>)` checks that the bytes are valid UTF-8. `bytes.to_vec()` copies them into a `Vec<u8>`.
- It returns a tuple, which the tests destructure with `let (status, body) = ...`.

---

## The tests

### Auth: missing token

```rust
#[tokio::test]
async fn protected_routes_reject_missing_token() {
    let (_dir, path) = temp_log();
    let app = app(&path);

    for (method, uri) in PROTECTED {
        let (status, _) = send(&app, method.clone(), uri, None).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }
}
```

- `#[tokio::test]`: an async test with its own runtime.
- `let (_dir, path)`: keep the `TempDir` alive. The test still uses a temp log even though it should never reach storage. If auth were broken, a handler would write into the temp folder, not a real file.
- `let app = app(&path);`: the variable `app` shadows the function `app`. That's legal, but only works because the function isn't needed again in this test.
- `for (method, uri) in PROTECTED`: loops over the array **by value** (arrays implement `IntoIterator`), destructuring each tuple.
- `method.clone()`: `send` takes `Method` by value, and `Method` isn't `Copy` (it can hold custom method names on the heap). Cloning keeps `method` available for the message.
- `let (status, _)`: the body is ignored.
- The third argument to `assert_eq!` is a custom message. If one route fails, the output names it, e.g. `GET /log/last`.

### Auth: wrong token

The same loop with `Some("wrong-token")`. Both tests are needed: one checks the "header missing" path, the other the "compare fails" path.

### The full lifecycle

```rust
#[tokio::test]
async fn log_lifecycle_with_valid_token() {
    let (_dir, path) = temp_log();
    let app = app(&path);

    // Add two entries. Keep the returned text instead of guessing timestamps.
    let (status, first) = send(&app, Method::POST, "/log", Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK);
    ...
```

One test walks through the whole story: add, add, read all, read last, delete last, read all, clear, read all. Each step sits under a comment, and the step's assertions come right after it.

Points worth noticing:

- `first` and `second` are the **returned** entries. `assert_eq!(log, format!("{first}\n{second}\n"))` checks order and exact content without guessing the time.
- **Shadowing:** `let (status, log)` is reused for each step, instead of `status2`, `log3` and so on.
- Every step checks the **status first**, then the body.
- DELETE steps expect `StatusCode::NO_CONTENT` (204).

### Empty log, last entry

```rust
#[tokio::test]
async fn last_entry_on_empty_log_returns_ok_with_empty_body() {
    ...
    // Current behavior. Planned to become 404; update this test when it does.
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "");
}
```

This pins **current** behavior, even though it's planned to change. When the 404 change lands, this test must be updated along with it.

### Health is public

```rust
    let (status, _) = send(&app, Method::GET, "/health", None).await;
    assert_eq!(status, StatusCode::OK);
```

There's no token, and the response is still 200. This proves `/health` sits outside the auth layer.

### Storage failure becomes 500

```rust
#[tokio::test]
async fn storage_error_returns_internal_server_error() {
    let (_dir, path) = temp_log();
    // A directory at the log path can't be read as a file, so storage fails.
    fs::create_dir(&path).expect("directory should be creatable at log path");
    let app = app(&path);

    let (status, body) = send(&app, Method::GET, "/log", Some(TOKEN)).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    // Internal details stay in the server log, never in the response.
    assert_eq!(body, "");
}
```

- The trick: putting a **directory** at the log path makes `read_to_string` fail with an error other than `NotFound`. That forces a real storage error without mocking anything.
- It checks the whole error path from doc 07: an anyhow error, then `AppError`, then a 500 with an **empty body**. The empty-body assertion is a security check that no error details leak to the client.

---

## What's not covered (yet)

15 tests is not full coverage. For example, there's no test for `GET /` redirecting, for 405 on wrong methods, or for 404 on unknown paths. The tests are there to make `cargo mutants` report zero missed mutants, not to reach a coverage number.

---

## Try it

1. Run `cargo test --test routes -- --nocapture` and find the line the storage error test prints.
2. Write a test `root_redirects_to_health` that sends `GET /` and asserts `StatusCode::TEMPORARY_REDIRECT`. (Reading the `location` header would need `send` to return more, so keep it to the status.)
3. Remove `.clone()` from `method.clone()` and read the error.

## Quick revise

- Integration tests use the app as a library (`tally::`). `ServiceExt::oneshot` sends a request straight into a cloned `Router`, with no network.
- `send(&app, method, uri, Option<token>) -> (StatusCode, String)`: build the request with a builder, `oneshot`, save the status, then `to_bytes` + `from_utf8` for the body.
- `PROTECTED` lists every guarded `(Method, path)`. Auth tests loop over it with no token and with a wrong token, and expect 401.
- The lifecycle test uses the returned entries instead of guessed times, shadows `(status, log)` at each step, and always asserts the status.
- The empty `/log/last` test pins the current 200 + empty body (404 is planned).
- A directory at the log path forces a storage error, which must give 500 + an empty body (no leaked details).
- Every test uses `temp_log()` with `_dir` kept alive, even tests that never touch storage.
