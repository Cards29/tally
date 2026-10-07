---
tags: [concept, rust]
aliases: ["test", "unit test", "integration test", "assert_eq", "tokio::test", "TempDir"]
---

# Testing

> Concept · First seen in: [08. Storage](../../files/08-storage-log.md) · Prev: [async and tokio](async-and-tokio.md) · Next: [HTTP basics](../web/http-basics.md) · [Index](../../README.md)

Rust has testing built in. You don't need a framework: `cargo test` finds and runs every test.

## A test is a function

```rust
#[test]
fn adds() {
    assert_eq!(2 + 2, 4);
}
```

- `#[test]` marks the function as a test.
- A test **passes if it returns without panicking**. It **fails if it panics**: a failed `assert`, an `expect` on `Err`, an index out of bounds, anything.
- Each test runs on its own thread, and tests run **in parallel** by default. So tests must not share state, such as one log file. That's why every test here uses its own `TempDir`.

### Assertion macros

```rust
assert!(cond);                          // panics if false
assert!(cond, "message {x}");           // with a custom message
assert_eq!(actual, expected);           // panics if !=, printing both values
assert_ne!(a, b);                       // panics if ==
assert_eq!(status, StatusCode::OK, "{method} {uri}"); // extra context in the message
```

When `assert_eq!` fails, it shows both sides:

```
assertion `left == right` failed
  left: "first\n"
 right: "first\nsecond\n"
```

The repo's convention is `assert_eq!(actual, expected)`, so `left` is always what the code produced.

To use `assert_eq!`, the type must implement `PartialEq` (for `==`) and `Debug` (to print it).

### `expect` in tests

```rust
let contents = fs::read_to_string(&path).expect("log file should be readable");
```

If this step fails, the test fails with a clear message. In this repo, test messages read "X should Y", and `unwrap()` is never used.

## Unit tests

Unit tests live **inside** the source file they test, in a child module:

```rust
// bottom of src/storage/log.rs
#[cfg(test)]
mod tests {
    use super::*;          // everything from the parent, private items included

    #[test]
    fn show_last_returns_final_line() { ... }
}
```

- `#[cfg(test)]` compiles the module **only** for `cargo test`. Normal builds skip it.
- Because `tests` is a child module, it can call **private** functions like `split_last_line`. Child modules can see their parent's private items.
- Use unit tests for one module's logic, without HTTP.

## Integration tests

Integration tests live in the `tests/` folder. Each `.rs` file there is compiled as a **separate crate** that uses your library **from the outside**, the way a real user would:

```rust
// tests/routes.rs
use tally::{routes, state::AppState};   // only `pub` items are reachable
```

- They can only see `pub` items, and they need a `lib.rs`. A binary-only crate can't be imported. That's why this repo has `src/lib.rs`.
- Use them for behavior across modules: router + auth + handlers + storage + errors together.

| | Unit tests | Integration tests |
|-|------------|-------------------|
| Location | `#[cfg(test)] mod tests` in `src/` | `tests/*.rs` |
| Can see private items? | yes | no, only `pub` |
| Typical scope | One function or module | The whole app through its public API |
| In this repo | 9, in `storage/log.rs` | 6, in `tests/routes.rs` |

## Async tests

A plain `#[test]` can't be `async`. tokio provides an attribute that sets up a runtime for each test:

```rust
#[tokio::test]
async fn health_returns_ok_without_token() {
    let (status, _) = send(&app, Method::GET, "/health", None).await;
    assert_eq!(status, StatusCode::OK);
}
```

## Test helpers and fixtures

Helpers are normal functions in the test module or test file. This repo has:

- `temp_log()`, which returns `(TempDir, String)`: a fresh folder plus a log path inside it.
- In integration tests: `app(file_name)` and `send(...)`.

`tempfile::TempDir` deletes its folder when **dropped** (RAII). Keep it alive with a named binding:

```rust
let (_dir, path) = temp_log();   // `_dir` lives until the test ends
// let (_, path) = temp_log();   // the folder is deleted immediately
```

`#[track_caller]` on a helper makes a panic inside it report the **caller's** line. Without it, every failure would point at the helper itself.

## Running tests

```sh
cargo test                          # everything
cargo test clear_last               # only tests whose name contains "clear_last"
cargo test --test routes            # only tests/routes.rs
cargo test --lib                    # only unit tests
cargo test -- --nocapture           # show println!/eprintln! output from passing tests
cargo test -- --test-threads=1      # run tests one at a time
```

Output from passing tests is captured (hidden) by default. Failing tests show their output.

## What makes a good test (this repo's style)

- **Name the behavior**, without a `test_` prefix: `clear_last_keeps_earlier_lines`.
- **Arrange / act / assert**, separated by blank lines. Reading files back belongs to *assert*.
- **Seed known data** with `fs::write(&path, "first\nsecond\n")`.
- **Never assert exact timestamps.** Use the value the code returned, or check the format instead.
- **Pin the format string as a literal** in the test, rather than reusing the constant. If the test reused `TIME_FORMAT`, changing the constant would change the test along with it, and nothing would be caught.
- **Always assert the status**, not just the body. A 500 has an empty body too.
- **Test current behavior.** Never change app behavior just to make a test pass. When behavior changes on purpose, update the test.
- **Isolate**: a `TempDir` for every test, even tests that never touch storage. That way a broken auth layer can't write a real file.

## Mutation testing

`cargo mutants` checks the tests themselves: it changes the code and expects some test to fail. See [Cargo and code checks](../tooling/cargo-and-checks.md), section "`cargo mutants` (mutation testing)".

## Common errors

| Problem | Cause |
|---------|-------|
| `cannot find function` in `tests/` | The item isn't `pub`, or there's no `lib.rs` |
| `async fn` test never runs | You wrote `#[test]` where `#[tokio::test]` was needed. The compiler says ``async functions cannot be used for tests``. |
| A test passes alone but fails with others | Shared state, like the same file. Use a `TempDir`. |
| A file is missing during the test | You dropped the `TempDir` with `_` |

## Read more

- [The Rust Book, ch. 11: Writing Automated Tests](https://doc.rust-lang.org/book/ch11-00-testing.html)

## Quick revise

> [!TIP]
> - `#[test] fn name()`: it passes if it doesn't panic. Tests run in parallel, so isolate them.
> - `assert!`, `assert_eq!(actual, expected)`, `assert_ne!`, and `expect("x should y")`. Never `unwrap`.
> - Unit tests: `#[cfg(test)] mod tests { use super::*; }`. They can see private items.
> - Integration tests: `tests/*.rs`. Each file is a separate crate, sees only `pub` items, and needs `lib.rs`.
> - Async: `#[tokio::test]`.
> - `TempDir` is deleted on drop, so bind it as `_dir`. `#[track_caller]` makes helper panics point at the caller.
> - Filter tests with `cargo test name`. Show output with `-- --nocapture`.
> - Style: behavior names, arrange/act/assert, seeded data, no exact times, literal format strings, always check the status.
