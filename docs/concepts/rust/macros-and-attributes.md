---
tags: [concept, rust]
aliases: ["macro", "format!", "attribute", "derive", "doc comments", "cfg"]
---

# Macros and attributes

> Concept · First seen in: [02. main.rs and the module tree](../../files/02-main-and-modules.md) · Prev: [Pattern matching](pattern-matching.md) · Next: [async and tokio](async-and-tokio.md) · [Index](../../README.md)

## Macros

A **macro** is code that writes code. It runs at compile time and expands into ordinary Rust before type checking. You can always tell a macro call by the `!`:

```rust
println!("hi");    // a macro
println("hi");     // would be a normal function (and doesn't exist)
```

### Why not just use functions?

Macros can do things functions can't:

- **Take any number of arguments.** `println!("{a} {b} {c}")`. Rust functions have a fixed number of parameters.
- **Check things at compile time.** `format!("{x}")` fails to compile if there's no `x` in scope.
- **Generate whole items** (structs, impls, functions).

### Macros used in this repo

| Macro | Does |
|-------|------|
| `format!(...)` | Builds a `String` |
| `println!(...)` / `eprintln!(...)` | Prints a line to stdout / stderr |
| `writeln!(file, ...)` | Writes a formatted line to anything that implements `std::io::Write`. Returns a `Result`. |
| `assert_eq!(a, b)` | Panics in tests if `a != b`, and shows both values |
| `assert!(cond)` | Panics if `cond` is false |
| `vec![1, 2, 3]` | Builds a `Vec` |
| `matches!(v, Pattern)` | `true` if `v` matches the pattern |

### Format strings

These rules are the same for `format!`, `println!`, `eprintln!`, `writeln!` and `panic!`:

```rust
let name = "tally";
let port = 3000;

format!("{name} on {port}")         // inline arguments (this repo's style)
format!("{} on {}", name, port)     // positional
format!("{0} {0}", name)            // positional, by index
format!("{:?}", vec![1, 2])         // Debug: "[1, 2]"
format!("{:#?}", some_struct)       // Debug, pretty-printed on multiple lines
format!("{:#}", anyhow_error)       // "alternate" Display: for anyhow, the whole chain
format!("{port:>6}")                // right-aligned, 6 characters wide
format!("{{literal braces}}")       // double the braces to print a literal { }
```

- `{}` uses the `Display` trait: output meant for users.
- `{:?}` uses `Debug`: output meant for developers.

Inline arguments only accept plain names. `"{config.port}"` is a compile error, so for fields use `"{}", config.port`.

### Declaring macros

You can write your own macros with `macro_rules!`, but you rarely need to. This repo doesn't.

## Attributes

An **attribute** is metadata attached to an item, written `#[...]`. It tells the compiler or a tool what to do with that item.

| Attribute | Effect | In this repo |
|-----------|--------|--------------|
| `#[derive(Clone)]` | Automatically writes a trait impl | `AppState` |
| `#[tokio::main]` | Rewrites `async fn main` so it runs on the tokio runtime | `main.rs` |
| `#[tokio::test]` | Same, for an async test | `tests/routes.rs` |
| `#[test]` | Marks a function as a test | the unit tests |
| `#[cfg(test)]` | Compiles the item only when running tests | `mod tests` in storage |
| `#[track_caller]` | A panic inside reports the *caller's* line | the `temp_log()` helpers |
| `#[allow(lint)]` / `#[warn(...)]` / `#[deny(...)]` | Changes a lint's level for this one item | not used |

`#[...]` applies to the next item. `#![...]` (with a `!`) applies to the enclosing item, such as the whole crate when it's at the top of `lib.rs`.

### Kinds of macros behind attributes

- `derive` macros: `#[derive(Clone, Debug)]` generates `impl Clone for ...`. See [Traits and generics](traits-and-generics.md).
- **Attribute macros**: `#[tokio::main]` receives the whole function and outputs a rewritten version.
- **Built-in attributes**: `#[test]`, `#[cfg]`, `#[allow]`. These are handled by the compiler itself.

### `#[cfg(...)]`: conditional compilation

```rust
#[cfg(test)]
mod tests { ... }   // only exists when you run `cargo test`
```

Normal builds skip the whole block, so test helpers never reach the real binary. Other conditions include `#[cfg(target_os = "linux")]` and `#[cfg(feature = "x")]`.

## Doc comments

Doc comments are attributes in disguise. `/// text` is the same as `#[doc = "text"]`.

| Syntax | Documents |
|--------|-----------|
| `/// ...` | The next item (function, struct, field) |
| `//! ...` | The enclosing item (the module or crate it's written in) |
| `// ...` | Nothing. A normal comment that tools ignore. |

Doc comments are Markdown. The usual sections:

```rust
/// Returns the full log contents. A missing file counts as an empty log.
///
/// # Errors
/// Returns an error if the file exists but can't be read.
pub fn show_log(file_name: &str) -> Result<String> { ... }
```

- `# Errors`: when the function returns `Err`. clippy's pedantic lints ask for it on public functions returning `Result`.
- `# Panics`: when the function panics.
- `# Examples`: code blocks here are compiled and run by `cargo test` (called doctests).

Run `cargo doc --open` to see this crate's docs rendered as a website.

## Try it

1. Change `format!("failed to bind {addr}")` to `format!("failed to bind {adr}")` and read the compile error.
2. Run `cargo doc --no-deps --open` and find `show_log`.
3. Install `cargo-expand` (`cargo install cargo-expand`), then run `cargo expand --bin tally` to see what `#[tokio::main]` produces.

## Read more

- [`std::fmt` (format string syntax)](https://doc.rust-lang.org/std/fmt/)
- [The Rust Book, ch. 20.5: Macros](https://doc.rust-lang.org/book/ch20-05-macros.html)
- [Rust Reference: attributes](https://doc.rust-lang.org/reference/attributes.html)

## Quick revise

> [!TIP]
> - `name!(...)` is a macro: code that generates code at compile time. Macros can take any number of arguments and check format strings.
> - Formatting: `{x}` inline (plain names only), `{}` uses Display, `{:?}` uses Debug, `{:#}` is the alternate form, `{{` prints a literal brace.
> - `#[attr]` applies to the next item. `#![attr]` applies to the enclosing item.
> - Key attributes: `derive`, `tokio::main`, `test`, `cfg(test)`, `track_caller`, `allow`.
> - `///` documents the next item, `//!` the enclosing one. They're Markdown with `# Errors`, `# Panics` and `# Examples` sections, rendered by `cargo doc`.
