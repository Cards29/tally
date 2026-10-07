---
tags: [file-doc]
aliases: ["config.rs", "state.rs", "Config", "AppState"]
---

# 03. Config and state

> Synced at commit `7e4f19b` · Prev: [02. main.rs and the module tree](02-main-and-modules.md) · Next: [04. Routes](04-routes.md) · [Index](../README.md)

| File | Job |
|------|-----|
| `src/state.rs` | `AppState`: the data every request handler needs (log path, auth token) |
| `src/config.rs` | `Config`: reads environment variables once at startup and builds `AppState` plus the port |

New concepts in this doc:

- [Structs and `impl`](../concepts/rust/structs-and-impl.md)
- [Traits and generics](../concepts/rust/traits-and-generics.md) (only `derive(Clone)` here; the rest comes in doc 07)
- [Config and deployment](../concepts/web/config-and-deploy.md)

Concepts you've already seen and will meet again here: [`Result`](../concepts/rust/result-and-panics.md), [anyhow](../concepts/rust/anyhow.md), [closures](../concepts/rust/closures-and-option.md), [shadowing and `String`](../concepts/rust/ownership-and-strings.md).

---

## Why two structs?

- `AppState` is what the **running app** needs on every request. It gets cloned and handed to handlers and middleware.
- `Config` is everything read at **startup**. That's `AppState` plus `port`, which only `main` needs: once the socket is bound, no handler cares about the port.

Keeping `port` out of `AppState` means handlers can't see data they don't use.

---

## `src/state.rs`

```rust
/// Shared state that every handler and the auth middleware receive.
#[derive(Clone)]
pub struct AppState {
    /// Path of the log file.
    pub file_name: String,
    /// Bearer token that protected routes require.
    pub auth_token: String,
}
```

```rust
/// Shared state that every handler and the auth middleware receive.
```

A doc comment for the struct. `///` documents the item right after it.

```rust
#[derive(Clone)]
```

Asks the compiler to write `impl Clone for AppState` automatically. The generated `clone()` clones each field one by one, so here it makes two new `String`s.

Why does `AppState` need `Clone`? axum gives every request its **own copy** of the state, and `routes.rs` also clones it once for the middleware. axum's `with_state` requires `S: Clone`. Without the derive, `routes.rs` fails to compile with ``the trait bound `AppState: Clone` is not satisfied``.

Cloning two short strings per request is cheap. If the state ever got big (a database pool, a large cache), you'd wrap it in `Arc` so a clone only copies a pointer. Not needed now.

```rust
pub struct AppState {
```

Defines a **struct**: a named group of fields. `pub` lets other modules (and other crates, like the tests) name the type. See [Structs and `impl`](../concepts/rust/structs-and-impl.md).

```rust
    pub file_name: String,
    pub auth_token: String,
```

Two fields, each `pub`, so code outside this module can read and set them. The tests build an `AppState` directly with `AppState { file_name: ..., auth_token: ... }`, which only works because both fields are public.

They're `String`, not `&str`, because the struct **owns** its data: it outlives the function that created it, so it can't borrow from that function's local variables. See [Ownership, borrowing and strings](../concepts/rust/ownership-and-strings.md).

---

## `src/config.rs`

```rust
use std::env;

use anyhow::{Context, Result};

use crate::state::AppState;

/// Runtime settings read from environment variables.
pub struct Config {
    /// State shared with handlers and middleware.
    pub state: AppState,
    /// TCP port to listen on. Defaults to 3000.
    pub port: u16,
}

impl Config {
    /// Loads `.env` if present, then builds a `Config` from `FILE_NAME`, `AUTH_TOKEN` and `PORT`.
    ///
    /// # Errors
    /// Returns an error if `FILE_NAME` or `AUTH_TOKEN` is unset, or if `PORT` is not a valid `u16`.
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let file_name = env::var("FILE_NAME").with_context(|| "FILE_NAME must be set")?;
        let auth_token = env::var("AUTH_TOKEN").with_context(|| "AUTH_TOKEN must be set")?;
        let port: u16 = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .with_context(|| "PORT must be a number between 0 and 65535")?;

        Ok(Self {
            state: AppState {
                file_name,
                auth_token,
            },
            port,
        })
    }
}
```

### Imports

```rust
use std::env;
```

Imports the `env` **module** from the standard library, so the code can write `env::var(...)`. Importing the module rather than `std::env::var` itself keeps the call readable: `env::var` says where `var` comes from.

```rust
use anyhow::{Context, Result};
```

`Result` is anyhow's `Result<T>`. `Context` is the trait that provides `.with_context`. See [anyhow](../concepts/rust/anyhow.md).

```rust
use crate::state::AppState;
```

`crate::` starts from this crate's root (`lib.rs`), then goes to the `state` module, then to `AppState`.

### The struct

```rust
pub struct Config {
    pub state: AppState,
    pub port: u16,
}
```

A struct can contain other structs. `u16` is an unsigned 16-bit integer (0 to 65535), which is exactly the range of TCP port numbers. Choosing that type means an invalid port like `70000` can't even be stored.

### The `impl` block

```rust
impl Config {
```

An `impl` block attaches functions to a type. Everything inside belongs to `Config`.

```rust
    pub fn from_env() -> Result<Self> {
```

- No `self` parameter, so this is an **associated function**, called on the type: `Config::from_env()`. Constructors are usually written like this. Rust has no special constructor syntax; `new`, `from_env` and the like are just names people choose.
- `Self` means "the type this `impl` is for", here `Config`. If the type were renamed, `Self` would still be correct.
- It returns `Result<Self>`, because reading config can fail.

### Loading `.env`

```rust
        dotenvy::dotenv().ok();
```

- `dotenvy::dotenv()` looks for a `.env` file in the current directory (and its parents), and sets each `KEY=value` line as an environment variable. It never overwrites a variable that's already set.
- It returns a `Result`. There's no `.env` on Render, which is fine, so the error is thrown away on purpose. `.ok()` turns the `Result` into an `Option` and the `Option` gets ignored. That silences the "unused `Result`" warning while making the intent clear: "try, don't care if it fails".
- The function is called with its full path (`dotenvy::dotenv`) instead of being imported, because it's used only once.

See [Config and deployment](../concepts/web/config-and-deploy.md).

### Required variables

```rust
        let file_name = env::var("FILE_NAME").with_context(|| "FILE_NAME must be set")?;
```

- `env::var("FILE_NAME")` returns `Result<String, env::VarError>`. It's `Err` if the variable is missing or isn't valid Unicode.
- `.with_context(|| "FILE_NAME must be set")` adds the message.
- `?` makes `from_env` return the error right away. Startup stops here.

This is called **fail fast**: a missing setting crashes at startup with a clear message, instead of causing a confusing error on the first request.

```rust
        let auth_token = env::var("AUTH_TOKEN").with_context(|| "AUTH_TOKEN must be set")?;
```

The same, for the secret token.

### The optional port

```rust
        let port: u16 = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .with_context(|| "PORT must be a number between 0 and 65535")?;
```

A chain; read it top to bottom:

1. `env::var("PORT")` gives `Result<String, VarError>`.
2. `.unwrap_or_else(|_| "3000".to_string())`: if the variable isn't set, use `"3000"`. Now it's a plain `String`. The closure gets the error as its argument, and `|_|` ignores it. `unwrap_or_else` instead of `unwrap_or` means the default `String` only gets built when it's needed.
3. `.parse()` turns the text into a number and gives `Result<u16, ParseIntError>`. How does `parse` know the target type is `u16`? Through the annotation `let port: u16`. Rust's type inference works backwards from there. The alternative is the "turbofish": `.parse::<u16>()`.
4. `.with_context(...)` then `?`: `"abc"` or `"70000"` fails here with a clear message.

Note that `unwrap_or_else` here is not `unwrap()`. It never panics. It's a `Result` method that supplies a fallback value.

### Building the result

```rust
        Ok(Self {
            state: AppState {
                file_name,
                auth_token,
            },
            port,
        })
```

- `Self { ... }` builds a `Config`.
- `file_name,` is **field init shorthand**: when a variable has the same name as the field, `file_name: file_name` can be written as just `file_name`.
- The `String`s are **moved** into the struct. No copies are made.
- `Ok(...)` wraps it as success. There's no `;`, so this is the function's return value.

---

## Try it

1. Run `PORT=70000 cargo run`. Which step of the chain fails, and why can't `u16` hold it?
2. Remove `#[derive(Clone)]` from `AppState` and run `cargo build`. Read where and why it fails, then put it back.
3. Rewrite `.parse()` as `.parse::<u16>()` and remove `: u16` from the `let`. It still compiles. Put it back the way it was.

## Quick revise

> [!TIP]
> - `AppState` (`file_name` and `auth_token`, both `String`) is cloned for every request, so it derives `Clone`. `Config` = `AppState` + `port`, and is only used at startup.
> - `impl Type { fn f() -> Self }`: an associated function with no `self`, called as `Type::f()`. `Self` = the type itself.
> - `dotenvy::dotenv().ok()`: load `.env` if it exists, otherwise ignore. It never overrides variables already set.
> - Required variables: `env::var(...).with_context(...)?`, which fails fast at startup.
> - Optional variables: `.unwrap_or_else(|_| default)`. `.parse()` picks its target type from `let x: u16`, or from the turbofish `parse::<u16>()`.
> - Field init shorthand: write `name` instead of `name: name`.
