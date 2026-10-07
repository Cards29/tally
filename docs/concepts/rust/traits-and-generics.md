---
tags: [concept, rust]
aliases: ["trait", "generics", "derive", "From", "Into", "orphan rule", "blanket impl"]
---

# Traits and generics

> Concept · First seen in: [03. Config and state](../../files/03-config-and-state.md) (`derive(Clone)`), then fully in [07. Errors](../../files/07-error.md) · Prev: [Structs and `impl`](structs-and-impl.md) · Next: [Closures and `Option`](closures-and-option.md) · [Index](../../README.md)

## Traits: shared behavior

A **trait** names a set of methods that a type can provide. It's close to an *interface* in Java, C# or Go.

```rust
trait Describe {
    fn describe(&self) -> String;                 // required: each type must write this

    fn shout(&self) -> String {                   // provided: has a default body
        self.describe().to_uppercase()
    }
}

struct Dog;

impl Describe for Dog {
    fn describe(&self) -> String {
        "a dog".to_string()
    }
}

println!("{}", Dog.shout()); // "A DOG"
```

`impl Trait for Type` is called "implementing the trait". The type then has the trait's methods.

### Traits must be in scope

To call a trait's method, the trait must be imported, even if you never write its name:

```rust
use std::io::Write;        // without this, writeln!(file, ...) fails: no method `write_fmt`
use anyhow::Context;       // without this, there's no `.with_context`
use tower::ServiceExt;     // without this, there's no `.oneshot`
use subtle::ConstantTimeEq;// without this, there's no `.ct_eq`
```

This is on purpose: a type can implement many traits, and you only get the methods you asked for. **Extension traits** like `Context` and `ServiceExt` exist only to add methods to other types.

## Standard traits you'll see everywhere

| Trait | Gives | Derivable? |
|-------|-------|------------|
| `Clone` | `.clone()`, an explicit copy | yes |
| `Copy` | Copying on assignment instead of moving (small types only) | yes |
| `Debug` | `{:?}` formatting | yes |
| `Display` | `{}` formatting, and `.to_string()` for free | no, you write it |
| `PartialEq` / `Eq` | `==` | yes |
| `Default` | `T::default()` | yes |
| `From<T>` / `Into<T>` | Conversions | no |
| `std::error::Error` | Marks a type as an error | no |
| `Send` / `Sync` | Safe to move or share between threads | automatic |

### `derive`

For many standard traits, the obvious implementation is "do it for each field". `#[derive(...)]` writes that code for you:

```rust
#[derive(Clone, Debug, PartialEq)]
struct Point { x: i32, y: i32 }
```

It only works if every field implements the trait too. `AppState` derives `Clone` because `String` is `Clone`.

## Generics: code for many types

```rust
fn largest<T: PartialOrd>(items: &[T]) -> &T {
    let mut best = &items[0];
    for item in items {
        if item > best { best = item; }
    }
    best
}
```

- `<T>` declares a **type parameter**: a placeholder for some type.
- `T: PartialOrd` is a **trait bound**: "`T` can be any type, as long as it implements `PartialOrd`". Without the bound, `>` wouldn't compile, because not every type can be compared.

Generic types work the same way: `Option<T>`, `Result<T, E>`, `Vec<T>`, axum's `Router<S>` and `State<S>`.

Rust compiles a separate copy of a generic function for each type it's used with. This is called **monomorphization**. Generics are as fast as hand-written code for each type, with no runtime cost.

### `where` clauses

When bounds get long, move them after the signature:

```rust
fn show<T>(x: T) -> String
where
    T: Display + Clone,   // `+` means "both"
{
    x.to_string()
}
```

### `impl Trait` shorthand

```rust
fn print_it(x: impl Display) { println!("{x}"); }   // in argument position: same as <T: Display>
fn make() -> impl Display { 42 }                     // in return position: "some type that's Display"
```

## Generic impls and blanket impls

You can implement a trait for *many* types at once:

```rust
impl<T: Display> ToString for T { ... }   // std does this: any Display type gets .to_string()
```

That's a **blanket impl**. This repo has one in `src/error.rs`:

```rust
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
```

Read it as: "for **any** type `E` that can turn into an `anyhow::Error`, here is how to turn an `E` into an `AppError`". That covers `std::io::Error`, `anyhow::Error` itself, `ParseIntError`, and so on.

## `From` and `Into`

```rust
impl From<u8> for MyNum { fn from(x: u8) -> Self { ... } }

let a = MyNum::from(5u8);
let b: MyNum = 5u8.into();   // you get `Into` for free from `From`
```

- Implement `From`. `Into` comes automatically.
- Use `Into` in bounds (`E: Into<anyhow::Error>`), because it accepts slightly more types.
- **`?` uses `From`** to convert errors. That's why `?` on an `anyhow::Result` inside a handler that returns `Result<_, AppError>` just works.

## Associated types

Some traits have a type "slot" that each implementation fills in:

```rust
trait Iterator {
    type Item;                                  // each iterator decides what it yields
    fn next(&mut self) -> Option<Self::Item>;
}
```

`Future` has `type Output`. axum's error messages often mention these.

## The orphan rule

You may write `impl Trait for Type` only if **the trait or the type is defined in your crate**.

| Trait | Type | Allowed? |
|-------|------|----------|
| yours | yours | yes |
| foreign | yours | yes: `impl IntoResponse for AppError` |
| yours | foreign | yes |
| foreign | foreign | **no**: `impl IntoResponse for anyhow::Error` |

The rule stops two crates from writing conflicting impls for the same pair. The standard workaround is a **newtype**: wrap the foreign type in your own struct. That's exactly why `AppError(anyhow::Error)` exists. See [Structs and `impl`](structs-and-impl.md).

## Trait objects, briefly

`Box<dyn Trait>` holds "some value that implements `Trait`", where the exact type is only known at runtime. Calls go through a lookup table (a vtable). `anyhow::Error` is basically a smart `Box<dyn std::error::Error + Send + Sync>`. The planned `LogStore` trait may end up used this way, or through generics.

## In this repo

| Where | What |
|-------|------|
| `state.rs` | `#[derive(Clone)]` |
| `error.rs` | `impl IntoResponse for AppError` (a foreign trait on a local type), plus the blanket `impl<E> From<E> for AppError` |
| everywhere | Extension traits that must be imported: `Context`, `Write`, `ServiceExt`, `ConstantTimeEq` |
| handlers | They return types implementing `IntoResponse`. axum's `Handler` trait is implemented for async functions whose arguments are extractors. |

## Common compiler errors

| Error | Meaning |
|-------|---------|
| `E0277: the trait bound X: Y is not satisfied` | A type is missing a required trait. Derive it, implement it, or change the type. |
| `E0599: no method named f found ... items from traits can only be used if the trait is in scope` | Import the trait |
| `E0117: only traits defined in the current crate can be implemented for types defined outside of the crate` | The orphan rule. Use a newtype. |
| `E0119: conflicting implementations` | Two impls overlap |

## Try it

1. Write a `Describe` trait, implement it for two structs, and write `fn announce(x: &impl Describe)`.
2. In a scratch file, try `impl axum::response::IntoResponse for anyhow::Error { ... }` and read E0117.
3. Remove `use std::io::Write;` from `src/storage/log.rs` and build. Read the hint, then put the line back.

## Read more

- [The Rust Book, ch. 10: Generics, Traits, and Lifetimes](https://doc.rust-lang.org/book/ch10-00-generics.html)
- [The Rust Book, ch. 20.2: Advanced Traits](https://doc.rust-lang.org/book/ch20-02-advanced-traits.html)

## Quick revise

> [!TIP]
> - A trait is a named set of methods. `impl Trait for Type`. Methods can be required or have a default body.
> - You can only call a trait's methods when the trait is imported (extension traits: `Context`, `Write`, `ServiceExt`).
> - `#[derive(Clone, Debug, ...)]` generates the impl, field by field.
> - Generics: `<T: Bound>`, or `where T: A + B`, or `impl Trait`. They're compiled per type (monomorphization), so there's no runtime cost.
> - Blanket impl: `impl<T: X> Y for T`. Implement `From` and get `Into` free. `?` uses `From` to convert errors.
> - Orphan rule: either the trait or the type must be yours. Use a newtype to get around it.
