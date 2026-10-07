---
tags: [index]
aliases: ["Index", "Start here"]
---

# Tally: a Rust web backend, explained

Tally is a small web server written in Rust. My phone sends it an HTTP request every time I press a button, and the server writes the current time into a log file. Other requests read the log back or delete entries.

It is small on purpose. With about 700 lines it still has every part a real backend has: configuration, routing, authentication, error handling, storage, and tests. These docs walk through all of it, line by line, and explain each Rust and web concept the first time it appears.

> These docs are written **after** the code. They describe what the code does at a given commit. Each file doc says which commit it was synced at.

## How the docs are organized

- **`files/`**: one doc per part of the repo. Each shows the code and explains it line by line.
- **`concepts/`**: one doc per idea (ownership, traits, `Result`, middleware, ...). Each explains the idea from zero with small standalone examples, then links back to where this repo uses it.

Every doc ends with a **Quick revise** section: a short summary for when you learned it once and need a refresher.

## Reading in Obsidian

These docs work in Obsidian and on GitHub alike: links are plain relative Markdown links, and boxes like "Quick revise" use `> [!TIP]` callouts, which both render.

1. **Open:** choose "Open folder as vault" and pick `docs/`. Obsidian's settings folder (`docs/.obsidian/`) is gitignored.
2. **Graph colors:** open Graph view, then Settings, then Groups. Add one group per query, for example `tag:#rust`, `tag:#web`, `tag:#tooling` and `tag:#file-doc`, each with its own color. File docs and the concepts they use then show up as clusters.
3. **Local graph:** open any doc and run "Open local graph" from the command palette. It shows what links in and out of that doc.
4. **Quick switcher (Ctrl+O):** every doc has aliases, so typing `auth.rs`, `Option`, `orphan rule` or `timing attack` jumps straight to the right doc.
5. **Backlinks pane:** on a concept doc, it lists every file doc that uses that concept.
6. **Tags pane:** browse docs by `#concept`, `#rust`, `#web`, `#tooling` or `#file-doc`.

## Reading path 1: follow the code

Read the file docs in order. They follow the life of the program: build, then startup, then one HTTP request moving through the app, then the tests. When a file doc links a concept you don't know yet, read the concept doc, then come back.

| # | Doc | Covers |
|---|-----|--------|
| 01 | [Cargo and tooling config](files/01-cargo-and-tooling.md) | `Cargo.toml`, `clippy.toml`, `.cargo/mutants.toml`, `.gitignore` |
| 02 | [main.rs and the module tree](files/02-main-and-modules.md) | `src/main.rs`, `src/lib.rs`, `handlers.rs`, `middleware.rs`, `storage.rs` |
| 03 | [Config and state](files/03-config-and-state.md) | `src/config.rs`, `src/state.rs` |
| 04 | [Routes](files/04-routes.md) | `src/routes.rs` |
| 05 | [Auth middleware](files/05-middleware-auth.md) | `src/middleware/auth.rs` |
| 06 | [Handlers](files/06-handlers.md) | `src/handlers/health.rs`, `src/handlers/log.rs` |
| 07 | [Errors](files/07-error.md) | `src/error.rs` |
| 08 | [Storage](files/08-storage-log.md) | `src/storage/log.rs` and its unit tests |
| 09 | [Integration tests](files/09-tests-routes.md) | `tests/routes.rs` |

## Reading path 2: concepts first

If you prefer theory before code, read the concepts in this order. Each one builds on the ones before it.

**Rust**

1. [Modules and crates](concepts/rust/modules.md): `mod`, `pub`, `use`, `crate::`, `lib.rs` vs `main.rs`
2. [Ownership, borrowing and strings](concepts/rust/ownership-and-strings.md): moves, `&`, `String` vs `&str`, lifetimes, shadowing
3. [Structs and `impl`](concepts/rust/structs-and-impl.md): fields, methods, `Self`, `const`, newtypes
4. [Traits and generics](concepts/rust/traits-and-generics.md): `trait`, `impl Trait for Type`, `<T>`, `where`, `derive`, `From`/`Into`
5. [Closures and `Option`](concepts/rust/closures-and-option.md): `|x| ...`, `Some`/`None`, `map`, `and_then`
6. [`Result` and panics](concepts/rust/result-and-panics.md): `Ok`/`Err`, `?`, `unwrap`, `expect`
7. [anyhow](concepts/rust/anyhow.md): `anyhow::Result`, `with_context`, error chains
8. [Pattern matching](concepts/rust/pattern-matching.md): `match`, guards, tuples, destructuring
9. [Macros and attributes](concepts/rust/macros-and-attributes.md): `format!`, `#[...]`, doc comments
10. [async and tokio](concepts/rust/async-and-tokio.md): futures, `.await`, the runtime
11. [Testing](concepts/rust/testing.md): `#[test]`, unit vs integration tests

**Web backend**

1. [HTTP basics](concepts/web/http-basics.md): methods, status codes, headers, bodies
2. [axum](concepts/web/axum.md): routers, handlers, extractors, shared state
3. [Middleware and auth](concepts/web/middleware-and-auth.md): layers, bearer tokens, timing attacks
4. [Config and deployment](concepts/web/config-and-deploy.md): environment variables, `.env`, Render

**Tooling**

1. [Cargo and code checks](concepts/tooling/cargo-and-checks.md): `Cargo.toml`, clippy, rustfmt, `cargo test`, `cargo mutants`
