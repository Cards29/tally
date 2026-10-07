---
tags: [concept, rust]
aliases: ["struct", "impl", "Self", "const", "newtype", "builder pattern"]
---

# Structs and `impl`

> Concept · First seen in: [03. Config and state](../../files/03-config-and-state.md) · Prev: [Ownership, borrowing and strings](ownership-and-strings.md) · Next: [Traits and generics](traits-and-generics.md) · [Index](../../README.md)

## Structs: grouping data

A **struct** bundles related values under one name. It's like a class's fields in other languages, but without inheritance.

### Named-field structs

```rust
pub struct Config {
    pub state: AppState,
    pub port: u16,
}

let c = Config { state: s, port: 3000 };   // every field must be given
println!("{}", c.port);                    // read a field with a dot
```

There are no default values and no "uninitialized" fields. Every field is set when the struct is created.

**Field init shorthand:** when a variable has the same name as the field, you can skip the `field:` part:

```rust
let port = 3000;
let c = Config { state, port };   // same as Config { state: state, port: port }
```

**Struct update syntax:** copy the remaining fields from another value:

```rust
let c2 = Config { port: 8080, ..c }; // moves c.state into c2
```

### Tuple structs

These have fields without names. You access the fields by position: `.0`, `.1`, ...

```rust
pub struct AppError(anyhow::Error);
let e = AppError(some_error);
let inner = e.0;
```

### Unit structs

```rust
struct Marker;   // no fields at all; sometimes used as a type to hang trait impls on
```

### Mutability

Mutability belongs to the **variable**, not to individual fields:

```rust
let mut c = Config { ... };
c.port = 8080;   // fine, because c is `mut`
```

## `impl`: attaching behavior

Functions go in separate `impl` blocks. A type can have any number of them.

```rust
pub struct Counter {
    count: u32,
}

impl Counter {
    // associated function: no `self`, called as Counter::new()
    pub fn new() -> Self {
        Self { count: 0 }
    }

    // method taking &self: read-only access
    pub fn get(&self) -> u32 {
        self.count
    }

    // method taking &mut self: can change fields
    pub fn bump(&mut self) {
        self.count += 1;
    }

    // method taking self: consumes the value (moves it in)
    pub fn into_inner(self) -> u32 {
        self.count
    }
}

let mut c = Counter::new();
c.bump();               // Rust adds the &mut automatically: Counter::bump(&mut c)
println!("{}", c.get());
let n = c.into_inner(); // c is moved; you can't use it afterwards
```

| First parameter | Called as | Meaning |
|-----------------|-----------|---------|
| (no `self`) | `Type::f()` | Associated function. Often a constructor. |
| `&self` | `value.f()` | Borrows the value to read it |
| `&mut self` | `value.f()` | Borrows the value to change it |
| `self` | `value.f()` | Takes ownership. Often used to convert the value into something else. |

`Self` (capital S) is the type the `impl` is for. `self` (lowercase) is the value the method was called on.

### Naming conventions

- `new` is a constructor. Rust has no special constructor syntax; `new` is just a convention.
- `from_x` builds the value from an `x`: `Config::from_env`.
- `into_x` consumes `self` and turns it into an `x`: `response.into_body()`.
- `as_x` gives a cheap borrowed view: `s.as_bytes()`, `s.as_str()`.
- `to_x` makes a (possibly expensive) converted copy: `s.to_string()`.
- `is_x` returns a `bool`.

## `const`

A compile-time constant. The type annotation is required, and by convention the name is `SCREAMING_SNAKE_CASE`.

```rust
const TIME_FORMAT: &str = "%a, %b %d %Y %H:%M:%S";
const PROTECTED: [(Method, &str); 5] = [ ... ];
```

The value is pasted into every place it's used. `const` items can live at the module level, or inside an `impl`.

## The newtype pattern

This means wrapping one type in a tuple struct with a single field:

```rust
pub struct AppError(anyhow::Error);
pub struct Meters(f64);
```

Reasons to do it:

1. **Implement a foreign trait for a foreign type.** You can't `impl IntoResponse for anyhow::Error`, because both are from other crates (this is the "orphan rule"; see [Traits and generics](traits-and-generics.md)). `AppError` is your own type, so you can. This is exactly why `src/error.rs` exists.
2. **Type safety.** `Meters` and `Seconds` can't be mixed up, even though both wrap `f64`.
3. **Hide the inner type.** The field is private, so you control what callers can do with it.

The wrapper is free at runtime: same size, no extra cost.

## The builder pattern

Some types have many optional settings. Instead of a constructor with 10 parameters, you chain setter methods, then call a final method that builds the value:

```rust
let file = OpenOptions::new()   // the builder
    .append(true)               // each setter returns the builder again
    .create(true)
    .open(path)?;               // the final call produces the real thing (a File)

let request = Request::builder()
    .method(Method::GET)
    .uri("/log")
    .body(Body::empty())?;      // the final call builds the Request
```

This repo uses builders from std (`OpenOptions`) and from axum's `http` re-export (`Request::builder`).

## Common compiler errors

| Error | Meaning |
|-------|---------|
| `E0063: missing field` | You must set every field |
| `E0616: field is private` | The field isn't `pub` |
| `E0594: cannot assign to c.port, as c is not declared as mutable` | Add `let mut` |
| `E0599: no function or associated item named new` | It's a method, not an associated function, or it doesn't exist |
| `E0382: use of moved value` after `into_*` | Methods taking `self` consume the value |

## Try it

1. Write the `Counter` above, then call `c.into_inner()` twice. Read the error.
2. Build a `Config` with struct update syntax, then try to use the old one's `state`.

## Read more

- [The Rust Book, ch. 5: Structs](https://doc.rust-lang.org/book/ch05-00-structs.html)

## Quick revise

> [!TIP]
> - Named struct: `S { a, b }`. Tuple struct: `S(x)`, with fields `.0`, `.1`. Unit struct: `S;`. Every field must be set when creating one.
> - Field init shorthand: `{ port }`. Update syntax: `{ port: 1, ..old }`. Mutability is per variable, not per field.
> - `impl T { ... }`: no `self` = associated function, called `T::f()`. Methods take `&self`, `&mut self` or `self`.
> - `Self` = the type. `self` = the value.
> - Naming: `new`, `from_*`, `into_*` (consumes), `as_*` (cheap borrow), `to_*` (copy), `is_*`.
> - `const NAME: Type = value;`
> - Newtype `struct W(T);` lets you add traits to a foreign type and gives you type safety, at zero cost.
> - Builder: chain setters, then one final call (`open`, `body`).
