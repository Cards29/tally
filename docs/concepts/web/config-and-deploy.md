---
tags: [concept, web]
aliases: ["env vars", ".env", "dotenvy", "Render", "deploy"]
---

# Config and deployment

> Concept · First seen in: [03. Config and state](../../files/03-config-and-state.md) · Prev: [Middleware and auth](middleware-and-auth.md) · Next: [Cargo and code checks](../tooling/cargo-and-checks.md) · [Index](../../README.md)

## Why config lives outside the code

The same binary runs on your laptop and on Render. What differs between them is the port, the file path, and the secret token. If those were written into the code:

- every environment would need a different build
- the secret would end up in git, readable by anyone with access to the repo

The common rule, from the "Twelve-Factor App" guidelines: **put config in environment variables**. The code reads them at startup.

## Environment variables

Every process has a list of `KEY=value` strings, inherited from whatever started it.

```sh
export AUTH_TOKEN=abc   # set it for this shell and the programs it starts
PORT=8080 cargo run     # set it for this one command only
env | grep PORT         # list what's set
```

In Rust:

```rust
use std::env;

let token: Result<String, env::VarError> = env::var("AUTH_TOKEN");
```

It returns `Err` if the variable is unset (or isn't valid Unicode). Every value is text, so numbers have to be `.parse()`d.

### Required vs optional

| Kind | Pattern in this repo | If missing |
|------|----------------------|------------|
| Required (`FILE_NAME`, `AUTH_TOKEN`) | `env::var(..).with_context(..)?` | Startup fails with a clear message |
| Optional (`PORT`) | `.unwrap_or_else(\|_\| "3000".to_string())` | A default is used |

Validate everything **at startup** (fail fast). A server that starts with a broken config only fails later, in confusing ways.

## `.env` files

Typing `export` lines before every run gets tiresome. A `.env` file holds them instead:

```
FILE_NAME=dates.log
AUTH_TOKEN=some-long-random-string
PORT=3000
```

`dotenvy::dotenv()` reads this file and sets the variables, except any that are **already set**. Real environment variables always win, so in production (where there's no `.env`) nothing changes.

Rules:

- **Never commit `.env`.** It's listed in `.gitignore`. A leaked token in git history stays leaked, even after you delete the file.
- Commit a `.env.example` with fake values, if you want to document which variables exist.

## Secrets

- Generate tokens randomly and make them long, for example `openssl rand -hex 32`.
- Don't log them, print them, or put them in URLs. URLs end up in logs and browser history.
- Send them only over HTTPS. Over plain HTTP, anyone on the network path can read the `Authorization` header.

## Deploying on Render

Render builds the app from git and runs it. In outline:

1. You connect the repo and pick a branch (here, `main`).
2. **Build command**: typically `cargo build --release`.
3. **Start command**: runs the built binary from `target/release/`.
4. **Environment variables** are set in Render's dashboard: `FILE_NAME`, `AUTH_TOKEN`, and optionally `TZ`.
5. Render sets `PORT` itself and sends traffic to it. That's why `main.rs` reads `PORT` and binds `0.0.0.0` rather than `127.0.0.1`.
6. Render terminates HTTPS in front of the app. Your phone talks HTTPS to Render, and Render forwards plain HTTP to the server.

### Free tier limits that matter here

- **Spin-down:** after a period with no traffic, the service stops. The next request wakes it up, which takes a while (a "cold start").
- **The filesystem is temporary:** each spin-down or deploy starts from a fresh disk. **The log file gets wiped.** That's why the plan is to move storage to a cloud database.
- **The timezone is UTC:** `chrono::Local` uses the server's zone. Setting `TZ` (for example `TZ=Asia/Dhaka`) in Render's dashboard changes it. If the image has no timezone data, the POSIX form `TZ=<+06>-6` works instead. In that form the sign is inverted: `-6` means UTC+6.

### Logs

`eprintln!` writes to stderr. Render collects stdout and stderr into its log viewer. That's where `AppError` messages end up.

## Try it

1. Run `PORT=4000 cargo run` and `curl localhost:4000/health`.
2. Put a different `PORT` in `.env` than on the command line, and check which one wins.
3. Generate a token with `openssl rand -hex 32`.

## Read more

- [The Twelve-Factor App: Config](https://12factor.net/config)
- [dotenvy docs](https://docs.rs/dotenvy)
- [Render docs](https://render.com/docs)

## Quick revise

> [!TIP]
> - Config goes in environment variables, not code. One binary works everywhere, and secrets stay out of git.
> - `env::var` returns `Result<String, _>`. Parse numbers yourself. Required: `?` with context. Optional: a default value.
> - Validate at startup (fail fast).
> - `.env` + `dotenvy` is for local dev. Real variables always win. Never commit `.env`.
> - Secrets: random, long, never logged, HTTPS only.
> - Render: builds from git, sets `PORT`, terminates HTTPS. Bind `0.0.0.0`. The free tier wipes the disk and runs in UTC, so set `TZ`.
