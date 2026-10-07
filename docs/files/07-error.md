---
tags: [file-doc]
aliases: ["error.rs", "AppError"]
---

# 07. Errors

> Synced at commit `7e4f19b` · Prev: [06. Handlers](06-handlers.md) · Next: [08. Storage](08-storage-log.md) · [Index](../README.md)

`src/error.rs` defines `AppError`: the bridge between anyhow errors (from storage) and HTTP responses (what axum sends back). It's short, but it uses the most trait machinery of any file in the repo.

Read [Traits and generics](../concepts/rust/traits-and-generics.md) first. This file is its main real-world example. It also uses the newtype pattern from [Structs and `impl`](../concepts/rust/structs-and-impl.md) (section "The newtype pattern"), and `{:#}` from [anyhow](../concepts/rust/anyhow.md) (section "Printing an error").

---

## The problem

Handlers call storage, and storage returns `anyhow::Result<T>`. To use `?` in a handler, and have errors turn into responses, axum needs the error type to implement `IntoResponse`.

- `anyhow::Error` doesn't implement `IntoResponse`.
- You can't add that impl yourself either. Both the trait (axum's) and the type (anyhow's) come from other crates. That's the **orphan rule**, compile error E0117.

The solution: wrap `anyhow::Error` in a type **you** own, and implement the trait for that.

---

## The code

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Handler error that wraps any `anyhow::Error` and becomes a 500 response.
pub struct AppError(anyhow::Error);

/// Logs the full error chain to stderr and returns a bare 500.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        eprintln!("{:#}", self.0);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
```

---

## The newtype

```rust
pub struct AppError(anyhow::Error);
```

- A **tuple struct** with one field: the newtype pattern. It costs nothing at runtime: same size, same memory layout as the `anyhow::Error` inside.
- The struct is `pub`, but its field is **private** (there's no `pub` before `anyhow::Error`). Code outside this module can't build one with `AppError(e)` or read `.0`. The only way in is the `From` impl below, so every `AppError` is created the same way.

---

## Turning it into a response

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
```

- `impl IntoResponse for AppError` implements axum's (foreign) trait for this (local) type, which the orphan rule allows.
- `IntoResponse` has exactly one required method: `fn into_response(self) -> Response`.
- It takes `self` **by value**: converting into a response consumes the error.

```rust
        eprintln!("{:#}", self.0);
```

- `self.0`: the inner `anyhow::Error`.
- `{:#}`: anyhow's "alternate" format, which prints the **whole chain** on one line, like `failed to read /x/test.log: Is a directory (os error 21)`.
- `eprintln!` writes to **stderr**. On Render, that shows up in the service logs. This is the only place the error details ever go.

The positional `{:#}` with `self.0` is needed because inline arguments only accept plain names, not field access.

```rust
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
```

The client gets a bare **500** with an **empty body**. That's a security choice: error chains can contain file paths and system details that an outside client has no business seeing. The test `storage_error_returns_internal_server_error` asserts that the body is empty.

Because `StatusCode` implements `IntoResponse` too, this method just hands over to it.

---

## Converting any error into `AppError`

```rust
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
```

A **blanket impl**. Read it as: "for every type `E` that can be converted into an `anyhow::Error`, here is how to convert an `E` into an `AppError`."

- `impl<E>` declares a generic type parameter for the impl.
- `From<E> for AppError`: the trait being implemented, `From<E>`, with `AppError` as the target type.
- `where E: Into<anyhow::Error>`: the bound. It covers `anyhow::Error` itself, `std::io::Error`, `ParseIntError`, and every other standard error type that is `Send + Sync + 'static`.

```rust
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
```

- `err.into()` turns `E` into `anyhow::Error`. The target type comes from the context: `Self(...)` needs an `anyhow::Error`.
- `Self(...)` builds the `AppError`. That's allowed here because we're inside the module that defines the private field.

### Why this makes `?` work in handlers

```rust
pub async fn show_log(...) -> Result<String, AppError> {
    Ok(store::show_log(&state.file_name)?)
    //                                  ^ Err(anyhow::Error) → From::from → AppError
}
```

`?` calls `From::from` on the error. With this impl in place, any anyhow or std error can be `?`-ed inside a handler.

### Why `AppError` must not implement `std::error::Error`

If it did, `AppError` itself would satisfy `E: Into<anyhow::Error>`. Then this impl would also provide `From<AppError> for AppError`, which clashes with the standard library's built-in `impl<T> From<T> for T`. The compiler would reject the code with E0119 (conflicting implementations). So `AppError` stays a plain wrapper, not an `Error` type. That's fine, because nothing needs it to be one.

---

## The full path of an error

```
fs::read_to_string fails (io::Error: Is a directory)
  └► storage: .with_context(|| "failed to read {file_name}")  → anyhow::Error
      └► handler: `?` → From::from → AppError
          └► axum: Result::Err(AppError) → AppError::into_response()
              ├► stderr: "failed to read /tmp/.../test.log: Is a directory (os error 21)"
              └► client: HTTP 500, empty body
```

---

## Try it

1. Change `{:#}` to `{}` and run `cargo test storage_error -- --nocapture`. Compare what's printed. Then change it back.
2. Add `pub` before `anyhow::Error` in the struct. Does anything break? Why is the field private anyway?
3. Add `#[derive(Debug)]` to `AppError`, then write `impl std::error::Error for AppError {}` (it needs `Display` too). Read error E0119, then remove it all.

## Quick revise

> [!TIP]
> - `AppError(anyhow::Error)` is a newtype, so the orphan rule allows `impl IntoResponse for AppError`. Its field is private.
> - `into_response`: `eprintln!("{:#}", self.0)` logs the whole chain, then the client gets a bare 500 with an empty body. Details never leave the server.
> - `impl<E> From<E> for AppError where E: Into<anyhow::Error>` is a blanket impl, so `?` turns any std or anyhow error into an `AppError`.
> - `AppError` must not implement `std::error::Error`, or the blanket impl would conflict with `From<T> for T`.
> - Error path: io::Error, then context (anyhow), then `?` (AppError), then 500.
