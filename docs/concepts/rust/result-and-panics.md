# `Result` and panics

> Concept · First seen in: [02. main.rs and the module tree](../../files/02-main-and-modules.md) · Prev: [Closures and `Option`](closures-and-option.md) · Next: [anyhow](anyhow.md) · [Index](../../README.md)

Rust has **no exceptions**. Errors come in two kinds:

| Kind | Mechanism | Use it for |
|------|-----------|------------|
| **Recoverable** | Return a `Result` | Things that can go wrong in normal use: a missing file, bad input, the network |
| **Unrecoverable** | `panic!` | Bugs: something that "can't happen" just happened |

## `Result<T, E>`

```rust
enum Result<T, E> {
    Ok(T),  // success, holding a value of type T
    Err(E), // failure, holding an error of type E
}
```

A function that can fail says so in its return type, so the caller can't miss it:

```rust
fn parse_port(s: &str) -> Result<u16, std::num::ParseIntError> {
    s.parse::<u16>()
}
```

Rust also warns if you ignore a `Result`. It's marked `#[must_use]`:

```rust
std::fs::write("a.txt", "hi"); // warning: unused `Result` that must be used
```

### Handling it with `match`

```rust
match parse_port("3000") {
    Ok(port) => println!("port {port}"),
    Err(e) => println!("bad port: {e}"),
}
```

`match` can also look at *which* error happened, using a **guard** (`if ...`). This is real code from `src/storage/log.rs`:

```rust
match fs::read_to_string(file_name) {
    Ok(contents) => Ok(contents),
    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()), // missing file = empty log
    Err(e) => Err(e).with_context(|| format!("failed to read {file_name}")),
}
```

## The `?` operator

Most of the time you just want "if this failed, return the error to my caller". `?` does exactly that:

```rust
fn load() -> Result<u16, std::num::ParseIntError> {
    let port = "3000".parse::<u16>()?; // Err: return it now. Ok: unwrap the value.
    Ok(port + 1)
}
```

`expr?` is roughly the same as:

```rust
match expr {
    Ok(v) => v,
    Err(e) => return Err(From::from(e)),
}
```

Notice `From::from(e)`: `?` **converts** the error into the function's own error type. That's how one function can use `?` on I/O errors, parse errors and more, as long as each one can convert into its return error type. `anyhow::Error` accepts almost any error, and so does this repo's own `AppError` (doc 07).

Rules for `?`:

- It only works inside a function that returns `Result` (or `Option`, where `?` on `None` returns `None`).
- `main` can return `Result` too. If it returns `Err`, Rust prints the error and exits with code 1.

### Other useful methods

| Method | Does |
|--------|------|
| `.ok()` | Turns it into `Option<T>`, throwing the error away |
| `.map(f)` | Changes the `Ok` value |
| `.map_err(f)` | Changes the `Err` value |
| `.unwrap_or(v)` / `.unwrap_or_else(f)` | Gives a fallback value on error |
| `.is_ok()` / `.is_err()` | Tests which one it is |

`config.rs` uses `unwrap_or_else` to make `PORT` optional:

```rust
env::var("PORT").unwrap_or_else(|_| "3000".to_string())
```

`|_|` is a closure that ignores its argument, which here is the error.

## Panics

A **panic** means the program has hit a state it can't continue from. The current thread unwinds: its stack is unrolled and every value on it is dropped. The panic message and location are printed. If the main thread panics, the program exits.

```rust
panic!("this should never happen");
let v = vec![1, 2, 3];
v[10];                                   // panics: index out of bounds
```

In a web server, tokio catches a panic inside one request's task, so the other requests keep going. But that request just dies, and its client gets no proper error response.

### `unwrap()` and `expect()`

Both turn `Result<T, E>` or `Option<T>` into `T`, and **panic** on `Err` or `None`:

```rust
let n: u16 = "3000".parse().unwrap();
let n: u16 = "3000".parse().expect("hard-coded port should parse");
```

`expect` lets you write the message. Phrase it as what *should* be true, because the message is shown exactly when it wasn't:

```
thread 'main' panicked at src/x.rs:3:30:
hard-coded port should parse: ParseIntError { kind: InvalidDigit }
```

### When is panicking OK?

| Situation | Choose |
|-----------|--------|
| It can fail at runtime because of the outside world (files, network, user input, env vars) | `Result` and `?` |
| Tests: a failure *should* stop the test | `expect("... should ...")` |
| It can't fail unless there's a bug, and you can explain why | `expect("reason it can't fail")` |
| Quick prototypes | `unwrap()`, but this repo bans it |

This repo:

- uses `?` in app code
- uses `expect("... should ...")` in tests
- never uses `unwrap()`: clippy's `unwrap_used` lint flags it

The reasoning: `unwrap()` says nothing about *why* it should be safe. `expect` forces you to write that down.

### Test assertions are panics too

`assert_eq!(a, b)` panics when `a != b`. A test **passes if it returns without panicking**. So `expect` is the natural way to say "this step should work" in tests.

## Common compiler errors

| Error | Meaning |
|-------|---------|
| `E0277: the ? operator can only be used in a function that returns Result or Option` | Change the function's return type |
| `E0277: ? couldn't convert the error to X` | There's no `From` conversion. Use `map_err`, or use anyhow. |
| `E0308: expected u16, found Result<u16, _>` | You forgot `?` or the handling |
| `unused Result that must be used` (a warning) | You ignored a `Result`. Handle it, or write `let _ = ...` on purpose. |

## Try it

1. Write `fn f() -> Result<u16, std::num::ParseIntError> { let n: u16 = "abc".parse()?; Ok(n) }` and call it from `main` with `println!("{:?}", f())`. You'll see `Err(ParseIntError { kind: InvalidDigit })`.
2. Change it to `.expect("should be a number")` and run it. Compare the output with step 1.
3. Remove `?` and read the type error.

## Read more

- [The Rust Book, ch. 9: Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)

## Quick revise

- There are no exceptions. Recoverable errors use `Result<T, E>` (`Ok(T)` / `Err(E)`). Bugs use `panic!`.
- A `Result` must be used. Handle it with `match` (guards like `Err(e) if ...` can test the error kind), with `?`, or with methods like `unwrap_or_else`.
- `?`: return `Err` early, after converting it with `From`, or unwrap `Ok`. It only works in functions returning `Result` or `Option`.
- `unwrap()` and `expect(msg)` panic on `Err` or `None`. In this repo: no `unwrap()`, `expect("... should ...")` in tests, and `?` in app code.
- A test passes if it doesn't panic. `assert_eq!` panics on a mismatch.
