---
tags: [file-doc]
aliases: ["routes.rs", "router"]
---

# 04. Routes

> Synced at commit `7e4f19b` · Prev: [03. Config and state](03-config-and-state.md) · Next: [05. Auth middleware](05-middleware-auth.md) · [Index](../README.md)

`src/routes.rs` is the app's map: which URL and method goes to which function, and which routes need the token.

New concepts in this doc:

- [HTTP basics](../concepts/web/http-basics.md): methods, paths, status codes, redirects
- [axum](../concepts/web/axum.md): `Router`, `route`, method routers, `merge`, `with_state`
- [Middleware and auth](../concepts/web/middleware-and-auth.md): layers, `route_layer`, `from_fn_with_state`

---

## The code

```rust
use axum::{
    Router,
    middleware::from_fn_with_state,
    response::Redirect,
    routing::{get, post},
};

use crate::{
    handlers::{health, log},
    middleware::auth,
    state::AppState,
};

/// Builds the app router. `/log` routes require a bearer token; `/` and `/health` are public.
pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/", get(|| async { Redirect::temporary("/health") }))
        .route("/health", get(health::check));

    let protected = Router::new()
        .route(
            "/log",
            post(log::add_entry)
                .get(log::show_log)
                .delete(log::clear_all),
        )
        .route("/log/last", get(log::show_last).delete(log::clear_last))
        .route_layer(from_fn_with_state(state.clone(), auth::require_token));

    public.merge(protected).with_state(state)
}
```

The routing table it builds:

| Method | Path | Handler | Auth? | Success |
|--------|------|---------|-------|---------|
| GET | `/` | inline closure | no | 307 redirect to `/health` |
| GET | `/health` | `health::check` | no | 200 |
| POST | `/log` | `log::add_entry` | yes | 200 + the new entry |
| GET | `/log` | `log::show_log` | yes | 200 + the whole log |
| DELETE | `/log` | `log::clear_all` | yes | 204 |
| GET | `/log/last` | `log::show_last` | yes | 200 + the last entry |
| DELETE | `/log/last` | `log::clear_last` | yes | 204 |

---

## Imports

```rust
use axum::{
    Router,
    middleware::from_fn_with_state,
    response::Redirect,
    routing::{get, post},
};
```

A nested import from one crate:

- `Router`: the type that holds the routes.
- `from_fn_with_state`: turns a plain async function into middleware.
- `Redirect`: a response that tells the client to go to another URL.
- `get`, `post`: functions that build a **method router** for one HTTP method.

```rust
use crate::{
    handlers::{health, log},
    middleware::auth,
    state::AppState,
};
```

These import **modules** (`health`, `log`, `auth`), not functions. The code below then writes `log::add_entry`, which makes it obvious that the function is a handler from `handlers::log`. Note that `crate::middleware` is this project's module, not `axum::middleware`. They don't clash because the full paths differ.

---

## The function

```rust
/// Builds the app router. `/log` routes require a bearer token; `/` and `/health` are public.
pub fn router(state: AppState) -> Router {
```

It takes `AppState` **by value** (owned), because the router keeps it for its whole life. It returns `Router`, which is short for `Router<()>`: a router that needs no more state. That `()` is explained under `with_state` below.

It's a normal function, not `async`. Building routes doesn't do any I/O. `main.rs` calls it, and so do the tests.

---

## Public routes

```rust
    let public = Router::new()
```

An empty router. Each `.route(...)` returns a new router with one more route added. This is the builder pattern from [Structs and `impl`](../concepts/rust/structs-and-impl.md).

```rust
        .route("/", get(|| async { Redirect::temporary("/health") }))
```

- `.route(path, method_router)` attaches handlers to a path.
- `get(handler)` means "for GET requests, call `handler`". Any other method on `/` gets **405 Method Not Allowed**.
- The handler is a **closure** written right here: `|| async { ... }`.
  - `||` is a closure with no arguments.
  - `async { ... }` is an **async block**, which creates a future. axum handlers must be async, meaning they must return a future.
  - So: "a function that, when called, returns a future that produces a `Redirect`".
- `Redirect::temporary("/health")` builds a **307 Temporary Redirect** response with the header `Location: /health`. Browsers and `curl -L` then request `/health`. "Temporary" means the client shouldn't remember it. A 308 Permanent redirect can be cached forever, which is a pain to undo. See [HTTP basics](../concepts/web/http-basics.md).

```rust
        .route("/health", get(health::check));
```

Here the handler is a **named function** passed without calling it: `health::check`, with no `()`. Functions are values in Rust. The `;` ends the `let`.

A health endpoint lets Render, uptime monitors, and you check whether the server is alive, without needing a token.

---

## Protected routes

```rust
    let protected = Router::new()
        .route(
            "/log",
            post(log::add_entry)
                .get(log::show_log)
                .delete(log::clear_all),
        )
```

A method router can handle several methods on the same path. Start with `post(...)`, then chain `.get(...)` and `.delete(...)`. One path, `/log`, does three different things depending on the HTTP method. This is the REST style: the **path** names a resource (the log), and the **method** says what to do with it.

```rust
        .route("/log/last", get(log::show_last).delete(log::clear_last))
```

The same idea for the "last entry" resource.

```rust
        .route_layer(from_fn_with_state(state.clone(), auth::require_token));
```

This line protects every route defined **above it on this router**.

- `auth::require_token` is the middleware function (doc 05).
- `from_fn_with_state(state, f)` wraps that function as a **layer**, and gives it access to the state (it needs `auth_token`).
- `state.clone()`: the middleware gets its own copy, because the original `state` is still needed for `with_state` below. Without `.clone()`, `state` would be moved here, and the last line would fail with "use of moved value". This is why `AppState` derives `Clone`.
- `.route_layer` vs `.layer`: `route_layer` only runs the middleware when a route **matched**. A request to a path that doesn't exist, like `/nope`, gets a plain **404** and never reaches auth. `.layer` also wraps the router's fallback (the "no route matched" handler), so unknown paths can end up answered with 401 instead of 404. That's confusing for clients, and it hints which paths exist.

Order matters: a layer only wraps routes added *before* it. Routes added after `.route_layer(...)` would be unprotected.

See [Middleware and auth](../concepts/web/middleware-and-auth.md).

---

## Combining

```rust
    public.merge(protected).with_state(state)
```

- `public.merge(protected)` combines the two routers into one. The auth layer stays attached only to the protected routes. That's the reason for building two routers: it keeps `/` and `/health` public.
- `.with_state(state)` provides the `AppState` that handlers ask for with `State<AppState>`.

### Why `Router` changes type

Until `with_state`, the router's type is really `Router<AppState>`: "a router that still **needs** an `AppState`". The compiler infers that from the handlers, which ask for `State<AppState>`. Calling `.with_state(state)` supplies it, and the result is `Router<()>`: "needs nothing more". Only a `Router<()>` can be served, and `Router<()>` is what the return type `Router` means. If you forget `with_state`, you get a type error instead of a crash at runtime.

No `;` at the end, so this is the return value.

---

## Try it

1. Move the `/health` route into `protected`. Run the tests. Which test fails, and why?
2. Run the server and `curl -i localhost:3000/nope`. Then change `.route_layer(...)` to `.layer(...)`, run the server again, and repeat. Did the status change? Run the tests too. Then change it back.
3. Remove `.clone()` from `state.clone()` and read the compiler error.
4. `curl -i localhost:3000/` and look at the `location` header. Then try `curl -iL localhost:3000/`.

## Quick revise

> [!TIP]
> - `Router::new().route(path, method_router)`. Method routers chain: `post(a).get(b).delete(c)`. Unmatched methods get 405.
> - A handler can be a named function (`health::check`, passed without `()`) or a closure `|| async { ... }`.
> - `Redirect::temporary` gives 307 with a `Location` header.
> - Two routers: `public` and `protected`. `route_layer(from_fn_with_state(state.clone(), auth::require_token))` protects only the routes added before it. Unknown paths still get 404.
> - `merge` combines the routers. `with_state(state)` turns `Router<AppState>` into `Router<()>`, which can be served.
> - `state.clone()` because both the layer and `with_state` need their own copy.
