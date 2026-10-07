---
tags: [concept, rust]
aliases: ["mod", "pub", "use", "crate", "visibility"]
---

# Modules and crates

> Concept · First seen in: [02. main.rs and the module tree](../../files/02-main-and-modules.md) · Next: [Ownership, borrowing and strings](ownership-and-strings.md) · [Index](../../README.md)

## The problem

A program with everything in one file gets hard to read quickly. You want to split code into named parts, hide the internal details of each part, and say which parts depend on which. Rust does this with **crates** and **modules**.

## Crates

A **crate** is what the compiler builds in one go. There are two kinds:

- **binary crate**: has a `fn main()` and compiles to a program you run. Its root file is `src/main.rs`.
- **library crate**: has no `main`. Other code uses it. Its root file is `src/lib.rs`.

A package (a folder with a `Cargo.toml`) can have one library crate and any number of binary crates. When both `src/lib.rs` and `src/main.rs` exist, the binary uses the library by the package's name:

```rust
// src/main.rs, in a package named "tally"
use tally::routes;
```

Every dependency in `Cargo.toml` is a crate too. `anyhow`, `axum` and `tokio` are library crates.

### Why split lib and main?

Integration tests (in `tests/`) are compiled as separate crates. They can import a library, but not a binary. So the usual pattern is: put all the logic in `lib.rs`, keep `main.rs` tiny, and let both `main.rs` and `tests/` use the library.

## Modules

A **module** is a named namespace inside a crate. Modules nest into a tree whose root is the crate's root file.

### Inline modules

```rust
mod greetings {
    pub fn hello() -> String {
        "hello".to_string()
    }

    fn secret() {} // private: only code inside `greetings` can call it
}

fn main() {
    println!("{}", greetings::hello());
    // greetings::secret(); // error[E0603]: function `secret` is private
}
```

### File modules

You can also put a module's contents in its own file. Write `mod name;` (with a semicolon and no body), and the compiler looks for the file:

| Declared in | Line | Loads |
|-------------|------|-------|
| `src/lib.rs` | `mod config;` | `src/config.rs` |
| `src/lib.rs` | `mod handlers;` | `src/handlers.rs` |
| `src/handlers.rs` | `mod health;` | `src/handlers/health.rs` |

So a module's children live in a folder named after the module. The older style used `src/handlers/mod.rs` instead of `src/handlers.rs`. Both still work. This repo uses the newer one.

**Important:** a `.rs` file that no `mod` line names is not compiled at all. Its errors won't show up, and its code doesn't exist as far as the program is concerned.

## Visibility: `pub`

Everything is **private by default**: visible only inside the module where it's defined and that module's children.

| Written | Visible to |
|---------|------------|
| (nothing) | This module and its children |
| `pub` | Everyone who can see the parent module |
| `pub(crate)` | Anywhere in this crate, but not outside it |
| `pub(super)` | The parent module |

Struct fields have their own visibility:

```rust
pub struct User {
    pub name: String, // anyone can read and write it
    password: String, // only this module can
}
```

For something to be reachable from outside, every module on its path must be `pub` too. `tally::handlers::log::add_entry` needs `pub mod handlers`, `pub mod log`, and `pub fn add_entry`.

## Paths

You name items by their path through the module tree, with `::` between the parts:

| Path starts with | Means |
|------------------|-------|
| `crate::` | The root of the current crate |
| `super::` | The parent module |
| `self::` | The current module |
| a crate's name (`std::`, `axum::`, `tally::`) | That crate's root |

```rust
// inside src/handlers/log.rs
use crate::storage::log as store; // from the root: storage, then log
```

`as` renames on import. Here it avoids a clash: this file is already inside a module called `log`.

## `use`

`use` makes a path short within the current file:

```rust
use std::collections::HashMap;
let m: HashMap<String, i32> = HashMap::new();
```

Forms you'll see in this repo:

```rust
use anyhow::{Context, Result};          // several items from one path
use std::{fs::{self, OpenOptions}, io}; // nested; `self` imports `fs` itself
use crate::storage::log as store;      // renamed
use super::*;                           // everything from the parent module (common in tests)
```

### Style rules in this repo

- Group imports in this order, with blank lines between groups: `std`, then external crates, then `crate::`.
- Sort alphabetically, and merge imports from the same crate into one `use`.
- Sort `mod` lines alphabetically.
- Parent module files hold only `pub mod` lines.

## Common compiler errors

| Error | Usual cause |
|-------|-------------|
| `E0583: file not found for module` | `mod foo;` exists, but the file is missing or in the wrong place |
| `E0433: failed to resolve: use of undeclared crate or module` | Typo in the path, or a missing `mod` line or dependency |
| `E0603: ... is private` | Something on the path is missing `pub` |
| `E0432: unresolved import` | The path in `use` doesn't exist |
| `no method named X found` | The method comes from a trait that isn't imported |

## Try it

1. Create `src/scratch.rs` containing `fn broken() -> i32 { "no" }`, then run `cargo build`. It passes, because nothing loads the file. Add `mod scratch;` to `lib.rs` and build again. Then remove both.
2. In `src/lib.rs`, change `pub mod routes;` to `mod routes;` and build. Which file breaks, and why?

## Read more

- [The Rust Book, ch. 7: Packages, Crates, and Modules](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html)

## Quick revise

> [!TIP]
> - Crate: one unit of compilation. A binary crate has `main.rs`; a library crate has `lib.rs`. A binary uses its own library as `package_name::`.
> - Tests in `tests/` can import the library but not the binary, so keep logic in `lib.rs`.
> - `mod x;` loads `x.rs`. Inside `x.rs`, `mod y;` loads `x/y.rs`. Files without a `mod` line are not compiled.
> - Everything is private by default. `pub`, `pub(crate)` and `pub(super)` open it up. Every module on the path must be visible.
> - Paths: `crate::`, `super::`, `self::`, or a crate's name. `use` shortens a path, `as` renames, `{}` groups.
