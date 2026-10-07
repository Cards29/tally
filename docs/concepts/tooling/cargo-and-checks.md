# Cargo and code checks

> Concept · Used in: [01. Cargo and tooling config](../../files/01-cargo-and-tooling.md) · Prev: [Config and deployment](../web/config-and-deploy.md) · [Index](../../README.md)

## What Cargo is

Cargo is Rust's build tool and package manager in one program. Almost every Rust project is driven through it:

| Command | What it does |
|---------|--------------|
| `cargo new name` | Creates a new project |
| `cargo build` | Compiles into `target/debug/` |
| `cargo build --release` | Compiles with optimizations into `target/release/` |
| `cargo run` | Builds, then runs the binary |
| `cargo check` | Type-checks without producing a binary. Much faster than `build`. |
| `cargo test` | Builds and runs all the tests |
| `cargo add serde` | Adds a dependency to `Cargo.toml` |
| `cargo tree` | Shows the dependency tree |

If you have used other ecosystems: Cargo is roughly npm + webpack (JavaScript), or pip + setuptools (Python), but everything comes in one official tool.

## Packages, crates and the manifest

- A **crate** is one unit of compilation: a library or a binary. See [Modules and crates](../rust/modules.md).
- A **package** is a folder with a `Cargo.toml`. It holds one or more crates.
- `Cargo.toml` is the **manifest**: the package's metadata, dependencies and settings.
- `Cargo.lock` records the exact version of every dependency, including dependencies of dependencies. Cargo writes it for you. Never edit it by hand.

Default layout, which Cargo finds by convention without any config:

```
Cargo.toml
src/main.rs      -> binary crate
src/lib.rs       -> library crate (optional)
tests/*.rs       -> integration tests, each file is its own crate
target/          -> build output
```

### Version requirements

```toml
serde = "1.0.200"      # >=1.0.200, <2.0.0  (the default, also called "caret")
serde = "=1.0.200"     # exactly this version
serde = "~1.0.200"     # >=1.0.200, <1.1.0
```

Under `1.0`, the rule is stricter: `"0.8.9"` means `>=0.8.9, <0.9.0`, because in `0.x` versions a minor bump is allowed to break things.

### Features

Crates can have optional parts that you switch on:

```toml
tokio = { version = "1", features = ["full"] }
```

Features keep compile times and binary sizes down for people who don't need everything.

### `dependencies` vs `dev-dependencies`

`[dev-dependencies]` are only compiled for tests, examples and benchmarks. Test helpers like `tempfile` go there, so they never end up in the shipped binary.

## Code checks

Rust's compiler already catches many bugs. Four extra tools catch more. This repo requires all of them to pass before a change is done:

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo mutants          # after changing tests or storage logic
```

### rustfmt (`cargo fmt`)

The official code formatter. `cargo fmt` rewrites your files into the standard style. `cargo fmt --check` changes nothing; it only fails if a file *would* change. Because everyone uses the same formatter, nobody argues about style.

### clippy (`cargo clippy`)

A linter with hundreds of rules ("lints") that spot code that compiles but is buggy, slow, or not idiomatic. Example:

```rust
if x.len() == 0 { }   // clippy: use `x.is_empty()` instead
```

Taking the command apart:

- `--all-targets`: lint tests and examples too, not just the main code.
- `--`: everything after this goes to clippy itself, not to cargo.
- `-D warnings`: **deny** warnings, which turns every warning into an error so the command fails.

Each lint has a **level**:

| Level | Effect |
|-------|--------|
| `allow` | Off |
| `warn` | Prints a warning |
| `deny` | Compile error |
| `forbid` | Error, and code can't lower it with `#[allow]` |

You can set levels:

- for the whole project in `Cargo.toml`, under `[lints.clippy]`
- for one item in code, with an attribute like `#[allow(clippy::unwrap_used)]`

Some lints also take settings, which live in `clippy.toml`. `disallowed-methods` is one: it bans specific methods by their path.

### `cargo test`

Finds and runs every function marked `#[test]`, in both `src/` and `tests/`. It also runs code examples written in doc comments. See [Testing](../rust/testing.md).

### `cargo mutants` (mutation testing)

Ordinary coverage tells you which lines your tests *ran*. Mutation testing tells you which lines your tests actually *check*.

`cargo mutants` makes a small change to your code, for example:

- replaces `a == b` with `a != b`
- replaces a function body with `Default::default()`
- deletes a `!`

Then it runs the tests:

- If some test fails, the mutant is **caught**. Good: the tests noticed the change.
- If every test still passes, the mutant is **missed**. Some behavior is not checked by any test.

Missed mutants are listed in `mutants.out/missed.txt`. In this repo that file must be empty.

An **equivalent mutant** changes the code without changing the behavior, so no test can ever catch it. Exclude those in `.cargo/mutants.toml`.

## Common mistakes

- **Running clippy without `--all-targets`.** Problems in test code slip through.
- **Forgetting `--` before `-D warnings`.** Cargo tries to read `-D` as its own flag and errors out.
- **Committing without `cargo fmt`.** `cargo fmt --check` fails in review. Run `cargo fmt` before every commit.
- **Ignoring `Cargo.lock` in an app.** Builds on different machines can then pick different dependency versions.

## Try it

1. Run `cargo check`, then `cargo build`, and compare how long each takes.
2. Add some badly indented code, run `cargo fmt --check`, then `cargo fmt`.
3. Run `cargo mutants --list` to see every mutant it would try, without running them.

## Read more

- [The Cargo Book](https://doc.rust-lang.org/cargo/)
- [Clippy lint list](https://rust-lang.github.io/rust-clippy/master/)
- [cargo-mutants book](https://mutants.rs)

## Quick revise

- Cargo builds, runs, tests and manages dependencies. `Cargo.toml` is the manifest. `Cargo.lock` pins exact versions.
- `"1.2.3"` means `>=1.2.3, <2.0.0`. For `0.x`, a minor bump counts as breaking.
- `features` turn on optional parts of a crate. `dev-dependencies` are for tests only.
- `cargo fmt`: formatting. `cargo clippy`: lints. Lint levels are allow, warn, deny, forbid. `-D warnings` makes warnings fail the command.
- `cargo mutants` changes code to check that tests notice. Missed mutant = untested behavior. Equivalent mutant = untestable, so exclude it.
