# async and tokio

> Concept · First seen in: [02. main.rs and the module tree](../../files/02-main-and-modules.md) · Prev: [Macros and attributes](macros-and-attributes.md) · Next: [Testing](testing.md) · [Index](../../README.md)

## The problem: waiting

A web server spends most of its time **waiting**: for a client to send bytes, for the disk, for a database. Suppose each connection gets its own OS thread, which then blocks while it waits:

- Each thread costs memory (a stack, often around 2–8 MB of reserved space) plus scheduling work for the OS.
- 10,000 slow clients means 10,000 threads, and the machine struggles.

**Async** lets a few threads juggle thousands of waiting tasks. When a task would wait, it *gives the thread back* so another task can run, and picks up later when its data is ready.

## Futures

In Rust, an `async fn` doesn't run when you call it. It returns a **future**: a value that describes work which hasn't happened yet.

```rust
async fn fetch() -> u32 { 42 }

let fut = fetch();     // nothing has run yet; fut is a Future<Output = u32>
let n = fut.await;     // now it runs, and n = 42
```

Futures are **lazy**. A future nobody awaits never runs. The compiler warns: `unused implementer of Future that must be used`.

### `.await`

`.await` means: "run this future. If it has to wait, pause **me** here and let the runtime run something else. Resume me when the result is ready."

```rust
let listener = TcpListener::bind(&addr).await?;
//                                    ^^^^^^ pause until the OS answers
//                                          ^ then handle the error
```

You can only use `.await` inside an `async` function or block.

Under the hood, the compiler turns each `async fn` into a **state machine**: an enum with one state per `.await` point. That's why async in Rust is cheap. A paused task is just a small struct in memory, not a whole thread.

## Runtimes: who runs the futures?

Rust's standard library defines what a `Future` *is*, but ships **no runtime** to run them. You pick one. **tokio** is the most widely used:

- an **executor**: a small pool of threads (by default, one per CPU core) that run tasks
- a **reactor**: watches sockets and timers through the OS (`epoll` on Linux), and wakes a task when its I/O is ready
- async versions of blocking APIs: `tokio::net::TcpListener`, `tokio::fs`, `tokio::time::sleep`, ...

### `#[tokio::main]`

`main` can't be `async` by itself, because something has to start the runtime first. The attribute does it for you:

```rust
#[tokio::main]
async fn main() { ... }

// expands roughly to:
fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(async { ... });
}
```

`block_on` is the bridge between normal code and async code: it blocks the current thread until the future completes.

### Tasks: running things at the same time

```rust
let handle = tokio::spawn(async {
    // runs as its own task, alongside everything else
    42
});
let n = handle.await.expect("task should not panic");
```

`axum::serve` does this for you: **every incoming connection becomes its own task**. That's how one small server can handle many phones at once. A panic inside one task doesn't crash the server, only that task.

## The golden rule: don't block inside async

A task only gives up its thread at an `.await`. If a task calls something slow and **blocking** (`std::thread::sleep`, heavy computation, `std::fs` on a slow disk), it holds the thread the whole time. Every other task scheduled on that thread has to wait.

| Blocking (std) | Async (tokio) |
|----------------|---------------|
| `std::thread::sleep` | `tokio::time::sleep(...).await` |
| `std::fs::read_to_string` | `tokio::fs::read_to_string(...).await` |
| `std::net::TcpListener` | `tokio::net::TcpListener` |

For blocking work you can't avoid, use `tokio::task::spawn_blocking(|| ...)`. It runs the work on a separate thread pool meant for blocking code.

### This repo's choice

`src/storage/log.rs` uses **blocking** `std::fs` inside async handlers. That breaks the golden rule, but on purpose:

- The files are tiny, and the OS caches them, so each call takes microseconds.
- Traffic is one person pressing a button.
- It keeps the storage code simple to learn.

It would matter under real load. The planned move to a cloud database will bring a proper async client anyway.

## `Send` and `'static`, briefly

tokio may move a task between threads, so everything a task holds across an `.await` must be safe to send between threads (the `Send` trait). You'll see this error if you hold something like `Rc` or a `std::sync::MutexGuard` across an `.await`:

```
future cannot be sent between threads safely
```

For now, just recognize it. It comes up again with shared state.

## Try it

1. In a scratch async test, write `let _ = tokio::time::sleep(std::time::Duration::from_secs(1));` without `.await`, and time it. Then add `.await` and compare.
2. Remove `#[tokio::main]` from `main.rs` and read the error (`main function is not allowed to be async`). Then put it back.

## Read more

- [The Rust Book, ch. 17: Async and Await](https://doc.rust-lang.org/book/ch17-00-async-await.html)
- [tokio tutorial](https://tokio.rs/tokio/tutorial)

## Quick revise

- Async lets a few threads handle many waiting tasks: a task gives up its thread at each `.await`.
- `async fn` returns a lazy **future**. Nothing runs until something `.await`s it.
- Rust has no built-in runtime. tokio provides the executor, the reactor, and async I/O.
- `#[tokio::main]` builds the runtime and `block_on`s your async `main`.
- `tokio::spawn` creates a task. `axum::serve` spawns one task per connection.
- Never block inside async. Use tokio's APIs or `spawn_blocking`. This repo uses blocking `std::fs` on purpose, because the load is tiny.
- Values held across an `.await` must be `Send`.
