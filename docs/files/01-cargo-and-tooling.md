# 01. Cargo and tooling config

> Synced at commit `7e4f19b` · Next: [02. main.rs and the module tree](02-main-and-modules.md) · [Index](../README.md)

These four files are not Rust code, but they decide how the Rust code is built, checked and kept out of git:

| File | Job |
|------|-----|
| `Cargo.toml` | The project's manifest: name, version, dependencies, lint levels |
| `clippy.toml` | Settings for the clippy linter |
| `.cargo/mutants.toml` | Settings for the mutation tester |
| `.gitignore` | Files git must never track |

New concept here: [Cargo and code checks](../concepts/tooling/cargo-and-checks.md). Read it first if `cargo`, clippy or mutation testing are new to you.

---

## `Cargo.toml`

```toml
[package]
name = "tally"
version = "0.1.2"
edition = "2024"

[dependencies]
anyhow = "1.0.104"
axum = "0.8.9"
chrono = "0.4.45"
dotenvy = "0.15.7"
subtle = "2.6.1"
tokio = { version = "1.53.1", features = ["full"] }

[dev-dependencies]
tempfile = "3.27.0"
tower = { version = "0.5.3", features = ["util"] }

[lints.clippy]
unwrap_used = "warn"
```

TOML is a config format made of `[sections]` and `key = value` lines.

### `[package]`

```toml
[package]
name = "tally"
```

The name of the package. It also becomes the name of the **library crate**, which is why the integration tests can write `use tally::routes;`. See [Modules and crates](../concepts/rust/modules.md).

```toml
version = "0.1.2"
```

A [semantic version](https://semver.org): `MAJOR.MINOR.PATCH`. In this repo, only `dev` changes it: the patch number goes up by one after each merge into `main`. `1.0.0` will be the first proper release.

```toml
edition = "2024"
```

Rust editions are opt-in language updates that come out every few years (2015, 2018, 2021, 2024). An edition can change small rules, for example which words are reserved. Crates on different editions still work together. Code written for 2024 compiles with the 2024 rules.

### `[dependencies]`

These are the external crates (libraries) the app uses. Cargo downloads them from [crates.io](https://crates.io).

```toml
anyhow = "1.0.104"
```

Easy error handling: one error type that can hold any error and add context messages to it. See [anyhow](../concepts/rust/anyhow.md).

A version written as `"1.0.104"` means "`1.0.104` or any newer compatible version" (`>=1.0.104, <2.0.0`). The exact version that was picked is written into `Cargo.lock`, so every build uses the same one.

```toml
axum = "0.8.9"
```

The web framework. It gives you routing, request parsing and responses.

```toml
chrono = "0.4.45"
```

Dates and times. Storage uses it to get the current local time and format it.

```toml
dotenvy = "0.15.7"
```

Reads a `.env` file and loads its `KEY=value` lines as environment variables. It's only useful during local development. On Render, the variables are set in the dashboard instead.

```toml
subtle = "2.6.1"
```

Comparisons that run in **constant time**. The auth middleware uses it to check the token without leaking information through response timing.

```toml
tokio = { version = "1.53.1", features = ["full"] }
```

The **async runtime**: the engine that runs async code and handles network I/O. See [async and tokio](../concepts/rust/async-and-tokio.md).

This line uses the longer table form `{ ... }` because it also sets **features**. Features are optional parts of a crate that you switch on. `"full"` turns on all of tokio's parts (networking, timers, the `#[tokio::main]` macro, ...). That's simple, but it compiles more than this app needs.

### `[dev-dependencies]`

Crates that are only used by tests (and examples and benchmarks). They are not compiled into the real server binary.

```toml
tempfile = "3.27.0"
```

Creates temporary directories that are deleted automatically. Tests use them so they never touch a real log file.

```toml
tower = { version = "0.5.3", features = ["util"] }
```

tower is the library axum is built on. It defines a `Service` trait: "something that takes a request and returns a response, eventually". The `util` feature adds `ServiceExt::oneshot`, which the integration tests use to send one request straight into the router without starting a real server.

### `[lints.clippy]`

```toml
[lints.clippy]
unwrap_used = "warn"
```

Lint levels for clippy. `unwrap_used` is off by default. Here it is turned on as a warning, so every `.unwrap()` gets flagged. Since the checks run with `-D warnings` (turn warnings into errors), in practice this bans `unwrap()`. Code has to use `?` or `.expect("...")` instead. Why that matters is in [`Result` and panics](../concepts/rust/result-and-panics.md).

---

## `clippy.toml`

```toml
disallowed-methods = [
    { path = "anyhow::Context::context", reason = "use with_context for consistency" },
]
```

`Cargo.toml` sets *which* lints are on and how loud they are. `clippy.toml` holds *settings* for lints. `disallowed-methods` is a list of methods clippy should flag. Each entry is an inline table with:

- `path`: the full path to the method. `anyhow::Context::context` is the `context` method of the `Context` trait in the `anyhow` crate.
- `reason`: shown in the warning so the reader knows why.

anyhow has two ways to attach a message to an error: `.context("msg")` and `.with_context(|| "msg")`. This repo picks one and bans the other, so the code looks the same everywhere. The difference between the two is explained in [anyhow](../concepts/rust/anyhow.md).

---

## `.cargo/mutants.toml`

```toml
# main.rs only wires things together; it's not worth mutating
exclude_globs = ["src/main.rs"]
```

`cargo mutants` changes your code in small ways (each change is a "mutant") and checks that some test fails. `exclude_globs` lists files it should leave alone. `main.rs` only connects the parts together and has no tests, so mutating it would only produce noise.

```toml
# StatusCode::default() is 200 OK, so this mutant behaves the same as the real code
exclude_re = ["health\\.rs.*replace check -> StatusCode with Default"]
```

`exclude_re` skips mutants whose description matches a regular expression. One mutant replaces the body of `health::check` with `Default::default()`. For `StatusCode`, the default value *is* `200 OK`, so the mutant behaves exactly like the real code and no test could catch it. This is called an **equivalent mutant**. In the regex, `\\.` is an escaped dot: `\.` matches a literal `.`, and the backslash itself needs escaping inside a TOML string.

---

## `.gitignore`

```
/target
/.env
dates.log
/mutants.out*
```

| Line | Why it's ignored |
|------|------------------|
| `/target` | Build output from cargo. Large, and can be rebuilt any time. |
| `/.env` | Holds the real `AUTH_TOKEN`. Secrets must never be committed. |
| `dates.log` | The real log data. |
| `/mutants.out*` | Reports from `cargo mutants`. |

A leading `/` means "only at the repo root". `*` matches anything, so `/mutants.out*` also covers `mutants.out.old`.

`Cargo.lock` is **not** ignored. For a binary (an app you run), you commit the lock file so every build uses exactly the same dependency versions.

---

## Try it

1. Run `cargo tree --depth 1`. It prints each direct dependency and its version. Compare those versions with `Cargo.toml`.
2. Add `let x: Option<i32> = Some(1); x.unwrap();` inside any function, then run `cargo clippy --all-targets -- -D warnings`. Read the error, then remove the line.
3. Change `unwrap_used = "warn"` to `"allow"` and run clippy again. What changed? Put it back.

## Quick revise

- `Cargo.toml` has four sections: `[package]` (name, version, edition), `[dependencies]` (used by the app), `[dev-dependencies]` (used only by tests), and `[lints.clippy]` (lint levels).
- `"1.2.3"` means "compatible with 1.2.3". `Cargo.lock` pins the exact versions. Commit it for apps.
- `features = [...]` switches on optional parts of a crate.
- Lint levels go in `Cargo.toml`. Lint settings go in `clippy.toml`.
- In this repo, `unwrap()` and `anyhow`'s `.context()` are banned. Use `?`, `expect`, and `with_context`.
- `.cargo/mutants.toml` skips `main.rs` and one equivalent mutant.
- `.gitignore` keeps build output, secrets, real data and reports out of git.
