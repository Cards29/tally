# Closures and `Option`

> Concept · First seen in: [02. main.rs and the module tree](../../files/02-main-and-modules.md) · Prev: [Traits and generics](traits-and-generics.md) · Next: [`Result` and panics](result-and-panics.md) · [Index](../../README.md)

These two are taught together because `Option`'s most useful methods take closures as arguments.

## Closures

A **closure** is an anonymous function you write inline. Arguments go between `| |`:

```rust
let add = |a: i32, b: i32| a + b;
println!("{}", add(2, 3)); // 5

let greet = || "hi";        // no arguments
let double = |x| x * 2;     // parameter types are inferred from how it's used
let long = |x: i32| {       // a block body for more than one line
    let y = x + 1;
    y * 2
};
```

### Capturing the environment

Unlike an `fn`, a closure can use variables from the code around it:

```rust
let addr = String::from("0.0.0.0:3000");
let make_msg = || format!("failed to bind {addr}"); // borrows addr
```

There are three ways to capture, and the compiler picks the lightest one that works:

| Captures by | The closure implements | It can be called |
|-------------|------------------------|------------------|
| shared borrow `&` | `Fn` | Any number of times |
| mutable borrow `&mut` | `FnMut` | Any number of times, but not at the same time |
| move (takes ownership) | `FnOnce` | At least once; maybe only once |

`move |x| ...` forces the closure to take ownership of what it captures. You need this when the closure outlives the current scope, for example when it's sent to another thread or task.

### Closures as arguments

Functions accept closures through those traits:

```rust
fn call_twice<F: Fn() -> String>(f: F) -> String {
    f() + &f()
}
```

### Why `with_context(|| ...)` takes a closure

```rust
.with_context(|| format!("failed to bind {addr}"))
```

`format!` allocates a new `String`. If the message were passed directly, it would be built on *every* call, including the successful ones that never need it. Wrapped in a closure, it's only built when there actually is an error. This is called **lazy evaluation**.

The same pattern appears all over the standard library: `unwrap_or_else`, `ok_or_else`, `map_or_else`.

## `Option<T>`

Rust has no `null`. A value that might be missing has the type `Option<T>`:

```rust
enum Option<T> {
    Some(T), // there is a value
    None,    // there isn't
}
```

`Option` is an **enum**: a type whose value is exactly one of several **variants**. `Some` and `None` are so common that you can use them without importing anything.

Why is this better than null? The type tells you a value might be missing, and the compiler won't let you use an `Option<String>` as a `String`. You have to handle `None` first. The famous "null pointer exception" can't happen.

```rust
let header: Option<&str> = request.headers().get("x").and_then(|v| v.to_str().ok());
// header.len()   // error: no method `len` on Option<&str>
```

### Getting the value out

```rust
match maybe {                       // handle both cases explicitly
    Some(v) => println!("{v}"),
    None => println!("nothing"),
}

if let Some(v) = maybe { ... }      // only care about Some

let v = maybe.unwrap_or("default"); // fall back to a default
let v = maybe.unwrap_or_else(|| compute_default()); // the lazy version
let v = maybe.expect("should be set"); // panic if None (see the next concept)
```

### Combinators: chaining without `match`

Methods that transform an `Option` and give back a new value:

| Method | If `Some(x)` | If `None` |
|--------|--------------|-----------|
| `.map(f)` | `Some(f(x))` | `None` |
| `.and_then(f)` | `f(x)`, where `f` itself returns an `Option` | `None` |
| `.map_or(default, f)` | `f(x)` | `default` |
| `.is_some()` / `.is_none()` | `true` / `false` | `false` / `true` |
| `.is_some_and(f)` | `f(x)` | `false` |
| `.ok_or(err)` | `Ok(x)` | `Err(err)` |

`map` vs `and_then`: use `and_then` when the closure can fail too (it returns an `Option`). Otherwise `map` would give you `Option<Option<T>>`.

### In this repo

The auth middleware (doc 05) gets the token with a chain where every step might fail:

```rust
let token = request
    .headers()
    .get(header::AUTHORIZATION)                       // Option<&HeaderValue>: the header may be missing
    .and_then(|value| value.to_str().ok())            // Option<&str>: it may not be valid text
    .and_then(|value| value.strip_prefix("Bearer ")); // Option<&str>: it may not start with "Bearer "

let authorized = token.is_some_and(|t| /* compare t */);
```

If any step returns `None`, the rest are skipped and `token` is `None`. Written with `match`, this would be three nested levels.

Storage (doc 08) uses `map_or`:

```rust
let start = trimmed.rfind('\n').map_or(0, |i| i + 1);
// a newline found at index i -> the last line starts at i + 1
// no newline -> the whole thing is one line, starting at 0
```

`Result` has a `.ok()` method that turns `Result<T, E>` into `Option<T>` and throws away the error. That's how `value.to_str().ok()` fits into the chain above.

## Common compiler errors

| Error | Meaning |
|-------|---------|
| `E0308: expected &str, found Option<&str>` | You forgot to handle `None` |
| `E0373: closure may outlive the current function` | Add `move` |
| `E0599: no method named X found for enum Option` | You called a method of `T` on `Option<T>`. Unwrap or `map` first. |

## Try it

1. Write `let v: Option<i32> = Some(4);`, then print `v.map(|x| x * 2)`, `v.and_then(|x| if x > 5 { Some(x) } else { None })`, and `v.map_or(0, |x| x + 1)`. Predict each result first.
2. Rewrite the auth token chain as nested `match` blocks. Compare how long each version is.

## Read more

- [The Rust Book, ch. 13.1: Closures](https://doc.rust-lang.org/book/ch13-01-closures.html)
- [The Rust Book, ch. 6.1: `Option`](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html#the-option-enum-and-its-advantages-over-null-values)
- [`Option` API docs](https://doc.rust-lang.org/std/option/enum.Option.html)

## Quick revise

- Closure: `|args| body`. It can capture surrounding variables by `&`, by `&mut`, or by `move`. The matching traits are `Fn`, `FnMut`, `FnOnce`.
- Pass a closure to make work lazy: it only runs when needed (`with_context`, `unwrap_or_else`).
- `Option<T>` is `Some(T)` or `None`. There's no null. The compiler forces you to handle `None`.
- To get the value out: `match`, `if let`, `unwrap_or`, `expect`.
- `map` transforms the inside. `and_then` chains steps that can fail. `map_or` gives a default. `is_some_and` tests the inside. `.ok()` turns a `Result` into an `Option`.
