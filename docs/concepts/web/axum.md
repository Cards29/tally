---
tags: [concept, web]
aliases: ["Router", "handler", "extractor", "State", "IntoResponse"]
---

# axum

> Concept · First seen in: [04. Routes](../../files/04-routes.md) · Prev: [HTTP basics](http-basics.md) · Next: [Middleware and auth](middleware-and-auth.md) · [Index](../../README.md)

axum is a web framework from the tokio team. It sits on top of:

- **hyper**: the actual HTTP implementation (parsing bytes into requests)
- **tower**: a general model where a `Service` takes a request and returns a future of a response, and `Layer`s wrap services
- **tokio**: the async runtime

axum's job is to turn ordinary `async fn`s into those services, with routing and type-checked request parsing.

## The four pieces

```
Router ── maps (method, path) ──► Handler ── uses ──► Extractors (input)
                                     │
                                     └── returns ──► IntoResponse (output)
```

### 1. Router

```rust
let app = Router::new()
    .route("/health", get(health))
    .route("/items", get(list).post(create))
    .route("/items/{id}", get(show));      // {id} is a path parameter (axum 0.8 syntax)
```

- `get`, `post`, `delete`, ... build a `MethodRouter`. Chain them for several methods on one path.
- A request for a path with no route gets **404**. A known path with an unsupported method gets **405**.
- `.merge(other)` combines routers. `.nest("/api", other)` mounts a router under a prefix.

### 2. Handlers

A handler is an `async fn`:

- each argument is an **extractor**
- the return type implements **`IntoResponse`**

```rust
async fn health() -> StatusCode { StatusCode::OK }

async fn show_log(State(state): State<AppState>) -> Result<String, AppError> { ... }
```

axum implements its `Handler` trait for any function that fits this shape, for up to 16 arguments. If a function *doesn't* fit (an argument that isn't an extractor, a return type that isn't a response, a future that isn't `Send`), the error is a long and vague ``the trait bound `fn(...) {name}: Handler<_, _>` is not satisfied``. When you see that, check the arguments and the return type first.

### 3. Extractors: getting input

Each argument tells axum what to pull out of the request. The extraction is **typed**: a failure is answered automatically with a 4xx response, and your handler never runs.

| Extractor | Gives you |
|-----------|-----------|
| `State<S>` | The shared app state |
| `Path<T>` | Path parameters (`/items/{id}` gives `Path(id): Path<u32>`) |
| `Query<T>` | The query string parsed into a struct (needs serde) |
| `Json<T>` | The body parsed as JSON (needs serde) |
| `HeaderMap` | All the headers |
| `String` / `Bytes` | The raw body |
| `Request` | The whole request |

**Order rule:** extractors that consume the **body** (`String`, `Json`, `Request`) must be the **last** argument, because the body can only be read once. In the auth middleware, `request: Request` comes after `State`, and it is the request.

#### Destructuring in the parameter

```rust
async fn show_log(State(state): State<AppState>) -> ...
//                ^^^^^^^^^^^^  ^^^^^^^^^^^^^^^
//                pattern       type
```

`State<AppState>` is a tuple struct wrapping the state. Writing the pattern `State(state)` in the parameter unwraps it right away, so the function body uses `state` directly. See [Pattern matching](../rust/pattern-matching.md).

### 4. Responses: `IntoResponse`

Anything that implements `IntoResponse` can be returned:

| Return type | Response |
|-------------|----------|
| `StatusCode` | That status, with an empty body |
| `String` / `&'static str` | 200, `text/plain; charset=utf-8` |
| `(StatusCode, String)` | That status, with the text |
| `Redirect` | 3xx with a `Location` header |
| `Json<T>` | 200, `application/json` |
| `Result<T, E>` where both are `IntoResponse` | `Ok` gives T's response, `Err` gives E's response |
| `Response` | Fully manual |

`Result` is the key one. Handlers return `Result<String, AppError>`, use `?` inside, and `AppError`'s `IntoResponse` turns any failure into a 500. See doc 07.

## Shared state

```rust
#[derive(Clone)]
struct AppState { ... }

let app = Router::new()
    .route("/log", get(show_log))   // show_log asks for State<AppState>
    .with_state(state);             // provide it once here
```

- The state type must be `Clone + Send + Sync + 'static`. axum clones it for each request that extracts it.
- For big or shared mutable state, wrap it in `Arc<...>` (and `Mutex` if it changes), so a clone is just a pointer copy.
- The router's type parameter tracks what's missing: `Router<AppState>` still needs state, and `.with_state()` makes it `Router<()>`. Only `Router<()>` can be served. Forgetting `with_state` is a compile error, not a runtime crash.

## Serving

```rust
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
axum::serve(listener, app).await?;
```

`serve` accepts connections forever and spawns one tokio task per connection.

## Testing without a network

A `Router` is a tower `Service`, so tests can call it directly:

```rust
use tower::ServiceExt; // gives you .oneshot

let response = app.oneshot(request).await?;
```

No port, no HTTP client, and it's fast. See [09. Integration tests](../../files/09-tests-routes.md).

## Read more

- [axum docs](https://docs.rs/axum) (the module docs for `extract`, `response` and `middleware` are excellent)
- [axum examples](https://github.com/tokio-rs/axum/tree/main/examples)

## Quick revise

> [!TIP]
> - axum = routing + typed extractors + `IntoResponse`, on top of hyper, tower and tokio.
> - `Router::new().route("/p", get(a).post(b))`. `merge`, `nest`. Unknown path gives 404. Wrong method gives 405.
> - Handler: an `async fn` whose arguments are extractors and whose return type implements `IntoResponse`. The `Handler` trait error means one of those doesn't fit.
> - Extractors: `State`, `Path`, `Query`, `Json`, `HeaderMap`, `Request`. Body extractors go last. Failed extraction gives an automatic 4xx.
> - Destructure in the parameter: `State(state): State<AppState>`.
> - Responses: `StatusCode`, `String`, tuples, `Redirect`, `Result<T, E>`.
> - State: `Clone + Send + Sync + 'static`, provided with `.with_state()`, which turns `Router<S>` into `Router<()>`.
> - Tests: `ServiceExt::oneshot` calls the router directly.
