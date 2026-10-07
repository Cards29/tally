# 02. main.rs and the module tree

> Synced at commit `7e4f19b` · Prev: [01. Cargo and tooling config](01-cargo-and-tooling.md) · Next: [03. Config and state](03-config-and-state.md) · [Index](../README.md)

This doc covers the files that hold the app together:

| File | Job |
|------|-----|
| `src/main.rs` | Program entry point: load config, build the router, start the server |
| `src/lib.rs` | Root of the library crate: lists the top-level modules |
| `src/handlers.rs`, `src/middleware.rs`, `src/storage.rs` | Parent modules: each only lists its child modules |

New concepts in this doc, in the order they appear:

- [Modules and crates](../concepts/rust/modules.md)
- [`Result` and panics](../concepts/rust/result-and-panics.md)
- [anyhow](../concepts/rust/anyhow.md)
- [async and tokio](../concepts/rust/async-and-tokio.md)
- [Closures and `Option`](../concepts/rust/closures-and-option.md)
- [Ownership, borrowing and strings](../concepts/rust/ownership-and-strings.md)
- [Macros and attributes](../concepts/rust/macros-and-attributes.md)

That's a lot for 20 lines. `main.rs` touches almost everything once. Skim the concept docs now, or come back to them as each one shows up below.

---

## The big picture: one package, two crates

```
src/lib.rs   -> library crate "tally": all the real code
src/main.rs  -> binary crate: a short main() that uses the library
tests/       -> integration tests: also use the library
```

`main.rs` doesn't declare any modules itself. It imports from the library as `tally::...`, the same way an outside user would. That split exists so `tests/routes.rs` can import the router too: test files can import a library, but they can't import a binary. Details in [Modules and crates](../concepts/rust/modules.md).

---

## `src/lib.rs`

```rust
//! Logs button-press timestamps sent over HTTP to a plain text file.

pub mod config;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;
pub mod storage;
```

```rust
//! Logs button-press timestamps sent over HTTP to a plain text file.
```

A doc comment for the whole crate. `//!` documents the item it sits **inside**, here the crate. `///` documents the item that comes **after** it. `cargo doc` turns these comments into HTML pages. See [Macros and attributes](../concepts/rust/macros-and-attributes.md).

```rust
pub mod config;
```

- `mod config;` tells the compiler: "there is a module named `config`; load it from `src/config.rs`". A Rust file is **not** compiled just because it exists. It's only compiled if some `mod` line names it.
- `pub` makes the module visible from outside this crate. Without it, `main.rs` and `tests/` couldn't write `tally::config`.

The other six lines do the same for each module. They are sorted alphabetically (a repo convention).

---

## `src/handlers.rs`, `src/middleware.rs`, `src/storage.rs`

```rust
//! HTTP request handlers.

pub mod health;
pub mod log;
```

```rust
//! Request middleware.

pub mod auth;
```

```rust
//! File-backed log storage.

pub mod log;
```

These are **parent modules**. `lib.rs` says `pub mod handlers;`, so Rust loads `src/handlers.rs`. That file says `pub mod health;`, so Rust loads `src/handlers/health.rs`: a folder with the same name as the parent file.

The full module tree:

```
crate (lib.rs)
├── config            src/config.rs
├── error             src/error.rs
├── handlers          src/handlers.rs
│   ├── health        src/handlers/health.rs
│   └── log           src/handlers/log.rs
├── middleware        src/middleware.rs
│   └── auth          src/middleware/auth.rs
├── routes            src/routes.rs
├── state             src/state.rs
└── storage           src/storage.rs
    └── log           src/storage/log.rs
```

There are two modules named `log`: `handlers::log` and `storage::log`. That's fine, because a module's full path is what identifies it. In the repo, the parent files only hold `pub mod` lines and a one-line doc comment.

---

## `src/main.rs`

```rust
use anyhow::{Context, Result};

use tally::{config::Config, routes};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env().with_context(|| "failed to load config")?;
    let addr = format!("0.0.0.0:{}", config.port);

    let app = routes::router(config.state);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;

    axum::serve(listener, app)
        .await
        .with_context(|| "server error")?;

    Ok(())
}
```

### Imports

```rust
use anyhow::{Context, Result};
```

`use` brings names into scope, so you can write `Result` instead of `anyhow::Result`. The `{ }` imports several names from the same crate at once.

- `Result`: anyhow's `Result<T>`, which is short for `std::result::Result<T, anyhow::Error>`. It's either success holding a `T`, or failure holding any error. See [anyhow](../concepts/rust/anyhow.md).
- `Context`: a **trait**. `with_context` is one of its methods. You need the trait in scope to call its methods, even though the word `Context` never appears again in this file. If you delete this import, the compiler says `no method named with_context found` and suggests importing the trait. See [Traits and generics](../concepts/rust/traits-and-generics.md).

```rust
use tally::{config::Config, routes};
```

`tally` is the library crate, named after the package. This line imports:

- the `Config` struct from the `config` module
- the `routes` module itself, so the code below can call `routes::router(...)`

Imports are grouped: std first, then external crates, then this project, with a blank line between groups.

### The entry point

```rust
#[tokio::main]
```

An **attribute macro**. `main` can't be `async` on its own, because something has to *run* async code, and Rust doesn't ship an async engine. This attribute rewrites the function into roughly:

```rust
fn main() -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(async {
            // your async body here
        })
}
```

So it starts tokio's runtime, then runs your async body to completion on it. See [async and tokio](../concepts/rust/async-and-tokio.md).

```rust
async fn main() -> Result<()> {
```

- `async fn`: this function returns a **future**, a value that does the work later, when something awaits it.
- `-> Result<()>`: `main` may return an error. `()` is the **unit type**, meaning "no value". So: "success with nothing to report, or an error". If `main` returns `Err`, Rust prints the error and the process exits with code 1. See [`Result` and panics](../concepts/rust/result-and-panics.md).

### Loading config

```rust
let config = Config::from_env().with_context(|| "failed to load config")?;
```

Read it left to right:

1. `Config::from_env()` calls an **associated function** on `Config` (a function that belongs to the type itself, like a static method in other languages). It returns `Result<Config>`. The next doc covers it.
2. `.with_context(|| "failed to load config")`: if the result is an error, wrap it in an outer message. Then an error prints like `failed to load config: AUTH_TOKEN must be set: environment variable not found`.
   - `|| "..."` is a **closure** with no arguments: a small inline function. anyhow only calls it when there's an error. See [Closures and `Option`](../concepts/rust/closures-and-option.md).
3. `?`: if the value is `Err`, return that error from `main` right now. If it's `Ok`, take the `Config` out and keep going.
4. `let config = ...` binds the `Config`. Variables are immutable by default.

### Building the address

```rust
let addr = format!("0.0.0.0:{}", config.port);
```

`format!` is a macro that builds a `String`. See [Macros and attributes](../concepts/rust/macros-and-attributes.md).

The repo normally uses inline arguments (`"{addr}"`), but inline arguments only accept plain variable names, not field access like `config.port`. So this line uses the positional `{}` form.

`0.0.0.0` means "listen on every network interface". `127.0.0.1` would only accept connections from the same machine, and Render couldn't route traffic to it.

### Building the app

```rust
let app = routes::router(config.state);
```

Builds the axum `Router`, which maps URLs to handler functions (doc 04).

`config.state` **moves** the `AppState` out of `config` and into `router`. After this line, `config.state` can't be used again. `config.port` still can: Rust tracks moves field by field. See [Ownership, borrowing and strings](../concepts/rust/ownership-and-strings.md).

### Opening the socket

```rust
let listener = tokio::net::TcpListener::bind(&addr)
    .await
    .with_context(|| format!("failed to bind {addr}"))?;
```

- `tokio::net::TcpListener::bind(&addr)` asks the OS for a TCP socket on that address. The type is written out with its full path instead of being imported, because it's used only once.
- `&addr` **borrows** the string: `bind` gets to read it without taking ownership. `addr` stays usable afterwards, which the next line relies on.
- `.await` pauses `main` until binding finishes. While `main` waits, the runtime can do other work.
- `.with_context(|| format!("failed to bind {addr}"))`: here the closure builds a `String`. This is why the repo prefers `with_context` over `context`: `format!` only runs if there's an error. Binding fails, for example, when the port is already in use.
- `?` returns the error from `main`, or unwraps the `TcpListener`.

### Serving

```rust
axum::serve(listener, app)
    .await
    .with_context(|| "server error")?;
```

`axum::serve` accepts connections on `listener` and hands each request to `app`. Awaiting it runs the server **forever**. It only returns if the server hits a fatal error. Each connection is handled as its own tokio **task**, so many requests are served at the same time.

### Done

```rust
Ok(())
```

The last expression in a function, written without a `;`, is its return value. `Ok(())` means "succeeded, nothing to return". In practice this line is only reached if `serve` returns without an error, which normally doesn't happen.

---

## What happens when you run it

1. Rust calls `main`. `#[tokio::main]` starts the runtime.
2. `Config::from_env()` reads `.env` and the environment variables. If anything is missing, `main` returns `Err`, the error is printed, and the process exits with code 1.
3. `router` builds the routes.
4. `bind` opens the port.
5. `serve` loops forever: accept a connection, then route the request to a handler.

---

## Try it

1. Run `PORT=abc cargo run`. (`dotenvy` never overrides a variable that is already set, so your `PORT=abc` wins over `.env`.) Read the error chain. Which part comes from which `with_context`, and which part comes from Rust's number parser?
2. Change `format!("0.0.0.0:{}", config.port)` to `format!("0.0.0.0:{config.port}")`. Read the compiler error, then change it back.
3. Delete `Context` from the first `use` line and read the compiler error. Then put it back.
4. Run the server twice in two terminals. The second one fails. What does the error say?

## Quick revise

- One package, two crates: `lib.rs` (library `tally`, all the code) and `main.rs` (binary, uses `tally::`). Tests can only import the library.
- `mod x;` loads `x.rs`. In `x.rs`, `mod y;` loads `x/y.rs`. A file isn't compiled unless some `mod` line names it. `pub` makes it visible outside.
- `//!` documents the enclosing item. `///` documents the next item.
- `#[tokio::main]` starts the async runtime so `main` can be `async`.
- `main` returns `anyhow::Result<()>`. An `Err` gets printed and the process exits with code 1.
- `?` means: return the error now, or unwrap the `Ok` value. `with_context(|| ...)` adds a message, and the closure runs only on error.
- Trait methods need the trait in scope (`use anyhow::Context`).
- Inline format arguments only take plain variable names, not `config.port`.
- `config.state` moves out of the struct. `&addr` borrows.
- `axum::serve(...).await` runs forever.
