---
tags: [file-doc]
aliases: ["auth.rs", "require_token"]
---

# 05. Auth middleware

> Synced at commit `7e4f19b` · Prev: [04. Routes](04-routes.md) · Next: [06. Handlers](06-handlers.md) · [Index](../README.md)

`src/middleware/auth.rs` holds one function, `require_token`. It runs before every `/log` handler and rejects requests that don't carry the right token.

New concept in this doc:

- [Pattern matching](../concepts/rust/pattern-matching.md) (here: destructuring `State(state)` in a parameter)

Background you need: [Middleware and auth](../concepts/web/middleware-and-auth.md) (the onion model, bearer tokens, timing attacks), and [Closures and `Option`](../concepts/rust/closures-and-option.md) (the `and_then` chain).

---

## The code

```rust
use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use subtle::ConstantTimeEq;

use crate::state::AppState;

/// Rejects the request with 401 unless `Authorization: Bearer <token>` matches `AUTH_TOKEN`.
///
/// The comparison runs in constant time, so response timing does not leak the token.
pub async fn require_token(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    let authorized =
        token.is_some_and(|t| bool::from(t.as_bytes().ct_eq(state.auth_token.as_bytes())));

    if !authorized {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    next.run(request).await
}
```

---

## Imports

```rust
use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
```

- `Request`: the whole HTTP request (method, URI, headers, body).
- `State`: the extractor that hands over the shared `AppState`.
- `StatusCode`: an HTTP status code, with constants like `StatusCode::UNAUTHORIZED`.
- `header`: a module of constants for standard header names, like `header::AUTHORIZATION`. Constants avoid typos in string names.
- `Next`: the rest of the middleware stack plus the handler, "everything inside this layer".
- `IntoResponse`: the trait. It's imported so `.into_response()` can be called.
- `Response`: the concrete response type, which is this function's return type.

axum re-exports the `http` crate as `axum::http`, so there's no need to add `http` as a separate dependency.

```rust
use subtle::ConstantTimeEq;
```

A trait. Importing it adds the `.ct_eq()` method to byte slices.

---

## The signature

```rust
pub async fn require_token(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
```

axum calls this function the same way it calls a handler, filling in each argument:

- `State(state): State<AppState>`: the extractor `State<AppState>` is a tuple struct wrapping the state. The **pattern** `State(state)` unwraps it on the spot, so the body can use `state.auth_token` instead of `state.0.auth_token`. See [Pattern matching](../concepts/rust/pattern-matching.md). The state comes from `from_fn_with_state(state.clone(), ...)` in `routes.rs`.
- `request: Request`: the request itself, owned. It must come after `State`, because it holds the body. A body extractor must be last among the extractors.
- `next: Next`: always the final argument of a `from_fn` middleware.
- `-> Response`: middleware returns a concrete `Response`. Both branches below produce one.

---

## Reading the token

```rust
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
```

Each step can fail, so each step returns an `Option`. The first `None` ends the chain. The final type is `Option<&str>`.

| Step | Type after it | Becomes `None` when |
|------|---------------|---------------------|
| `request.headers()` | `&HeaderMap` | (never) |
| `.get(header::AUTHORIZATION)` | `Option<&HeaderValue>` | The header is missing |
| `.and_then(\|value\| value.to_str().ok())` | `Option<&str>` | The header has bytes that aren't visible ASCII. `to_str` returns a `Result`, and `.ok()` makes it an `Option`. |
| `.and_then(\|value\| value.strip_prefix("Bearer "))` | `Option<&str>` | The value doesn't start with `"Bearer "`. Otherwise you get the rest of the string, which is the token. |

Notes:

- These are all **borrows** (`&`) pointing into `request`. Nothing is copied. Because `token` borrows from `request`, the borrow must end before `request` is moved into `next.run(request)`. It does: the last use of `token` is on the next line.
- `strip_prefix` is case-sensitive, and expects exactly one space. `bearer abc` would be rejected. That's stricter than the HTTP spec, which says the scheme name is case-insensitive, but fine for one client you control.
- The closures here use `value` in both steps. Each closure has its own scope, so the names don't clash.

---

## Checking it

```rust
    let authorized =
        token.is_some_and(|t| bool::from(t.as_bytes().ct_eq(state.auth_token.as_bytes())));
```

From the inside out:

1. `t.as_bytes()` and `state.auth_token.as_bytes()`: both strings as `&[u8]` byte slices.
2. `.ct_eq(...)` compares them in **constant time** and returns `subtle::Choice`.
3. `bool::from(choice)` converts the `Choice` into a normal `bool`.
4. `token.is_some_and(|t| ...)`: if there's no token, the result is `false`. If there is one, it's whatever the closure returns.

So `authorized` is `true` only if a token was sent **and** it matches. Why `ct_eq` instead of `==`? A plain `==` stops at the first wrong byte, so the response time would leak how many leading bytes were right. See [Middleware and auth](../concepts/web/middleware-and-auth.md), section "Timing attacks".

---

## Rejecting or passing on

```rust
    if !authorized {
        return StatusCode::UNAUTHORIZED.into_response();
    }
```

- `!` is boolean NOT.
- `return` exits early. This is a **guard clause**: handle the failure case first, then the main path below doesn't need to be nested inside an `else`.
- `StatusCode::UNAUTHORIZED` is 401. `.into_response()` (from the `IntoResponse` trait) turns it into a `Response` with an empty body.
- The handler **never runs**. Storage is never touched.

```rust
    next.run(request).await
}
```

Authorized, so call the rest of the stack: the actual handler. `request` is **moved** into `run`. `.await` waits for the handler's response, and that response is returned unchanged. There's no `;`, so it's the return value.

---

## What the tests check

`tests/routes.rs` sends every protected method and path with no token, then with a wrong token, and expects 401 every time. Doc 09 walks through it.

---

## Try it

1. `curl -i -X POST localhost:3000/log` gives 401. Add `-H "Authorization: Bearer $TOKEN"` and you get 200.
2. Try `-H "Authorization: bearer $TOKEN"` (lowercase `b`). What happens, and which line decides it?
3. Replace the `ct_eq` line with `token == Some(state.auth_token.as_str())`. The tests still pass. Why can't tests catch a timing leak? Then put the original back.

## Quick revise

> [!TIP]
> - Middleware arguments go in this order: `State(state): State<AppState>`, then `request: Request`, then `next: Next`. It returns a `Response`.
> - `State(state)` in the parameter is a pattern that unwraps the extractor.
> - The token comes from an `Option` chain: `headers().get(AUTHORIZATION)`, then `to_str().ok()`, then `strip_prefix("Bearer ")`. Any `None` means no token.
> - `is_some_and` + `ct_eq` + `bool::from` = present **and** equal, compared in constant time.
> - Guard clause: `if !authorized { return 401 }`. Otherwise `next.run(request).await` runs the handler.
> - Header constants (`header::AUTHORIZATION`) avoid typos in header names.
