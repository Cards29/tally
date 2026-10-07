---
tags: [concept, rust]
aliases: ["with_context", "error chain", "anyhow::Result"]
---

# anyhow

> Concept · First seen in: [02. main.rs and the module tree](../../files/02-main-and-modules.md) · Prev: [`Result` and panics](result-and-panics.md) · Next: [Pattern matching](pattern-matching.md) · [Index](../../README.md)

## The problem

With plain `Result<T, E>`, every function has to name one error type `E`. Real code fails in many different ways:

```rust
fn load() -> Result<Config, ???> {
    let text = std::fs::read_to_string("c.toml")?; // std::io::Error
    let port: u16 = text.trim().parse()?;          // std::num::ParseIntError
    ...
}
```

`???` has to be one type that both errors convert into. Without help, you'd write your own enum that wraps every possible error, plus a `From` impl for each one. For a library, that's often the right call, because callers may want to `match` on exact errors. For an **application**, where most errors just get logged, it's a lot of boilerplate.

## The solution: `anyhow::Error`

`anyhow::Error` is **one** error type that can hold *any* error. Any type that implements `std::error::Error + Send + Sync + 'static` converts into it automatically, so `?` just works:

```rust
use anyhow::Result; // = std::result::Result<T, anyhow::Error>

fn load() -> Result<u16> {
    let text = std::fs::read_to_string("c.toml")?; // io::Error becomes anyhow::Error
    let port: u16 = text.trim().parse()?;          // ParseIntError becomes anyhow::Error
    Ok(port)
}
```

`anyhow::Result<T>` is a type alias with the error type already filled in. You only write the success type.

### Making errors from scratch

```rust
use anyhow::{anyhow, bail};

return Err(anyhow!("port {port} is reserved")); // build an error from a message
bail!("port {port} is reserved");               // shorthand for the line above
```

## Context: messages that explain *what* failed

A raw `io::Error` says `No such file or directory (os error 2)`. Which file? Doing what? **Context** wraps the error in a higher-level message:

```rust
use anyhow::Context; // the trait that adds the methods below

let text = std::fs::read_to_string(path)
    .with_context(|| format!("failed to read {path}"))?;
```

Each wrap adds a layer, so errors form a **chain**, from the outermost message to the root cause:

```
failed to load config            <- added in main.rs
  AUTH_TOKEN must be set         <- added in config.rs
    environment variable not found   <- the original std::env::VarError
```

### `.context` vs `.with_context`

```rust
.context("failed to load config")                 // the message is built every time
.with_context(|| format!("failed to read {path}")) // the closure runs only on error
```

For a plain `&str`, the cost is the same. For `format!`, `.context(format!(...))` allocates a `String` even when everything succeeds. This repo **always** uses `with_context`, for consistency: `clippy.toml` bans `.context` through `disallowed-methods`. See [Closures and `Option`](closures-and-option.md) for why closures make the message lazy.

`Context` is a **trait**, and anyhow implements it for every `Result<T, E>` whose `E` is an error, and also for `Option<T>`. That's why the methods show up on any `Result` once the trait is imported. On an `Option`, `with_context` turns `None` into an error with your message.

### Message style in this repo

- lowercase, with no trailing period: `"failed to read {file_name}"`
- name the file when there is one
- use inline format arguments: `{file_name}`, not `{}` followed by `file_name`

## Printing an error

| Format | Shows |
|--------|-------|
| `{}` | Only the outermost message: `failed to load config` |
| `{:#}` | The whole chain on one line: `failed to load config: AUTH_TOKEN must be set: environment variable not found` |
| `{:?}` | The chain on multiple lines, plus a backtrace if enabled. This is what `main` prints when it returns `Err`. |

`src/error.rs` logs with `eprintln!("{:#}", self.0)` to get the one-line chain.

## anyhow vs thiserror

| | anyhow | thiserror |
|-|--------|-----------|
| Used for | Applications | Libraries |
| Error type | One opaque `anyhow::Error` | Your own enum, with a `#[derive(Error)]` |
| Can callers `match` on the exact error? | Hard (you'd need `downcast_ref`) | Easy |
| Boilerplate | Almost none | A little |

Tally is an app, and it only ever logs errors and returns 500, so anyhow fits.

## In this repo

- `config.rs`, `storage/log.rs` and `main.rs` return `anyhow::Result<T>`.
- `error.rs` wraps `anyhow::Error` in `AppError` so axum can turn it into an HTTP 500 response (doc 07).

## Try it

1. Run `PORT=abc cargo run` and read the full chain.
2. Change `eprintln!("{:#}", self.0)` in `error.rs` to `{}`, trigger an error, and compare the output. Then change it back.

## Read more

- [anyhow docs](https://docs.rs/anyhow)

## Quick revise

> [!TIP]
> - `anyhow::Error` holds any error. `anyhow::Result<T>` = `Result<T, anyhow::Error>`. `?` converts errors into it automatically.
> - `use anyhow::Context;` adds `.with_context(|| msg)` to `Result` and `Option`. It wraps the error in a message, building a chain.
> - `with_context` is lazy (it takes a closure). This repo bans `.context`.
> - `anyhow!("...")` creates an error. `bail!("...")` returns one.
> - Print with `{:#}` for a one-line chain, or `{:?}` for multiple lines.
> - anyhow is for apps. thiserror is for libraries whose callers need to match on errors.
