# 08. Storage

> Synced at commit `7e4f19b` · Prev: [07. Errors](07-error.md) · Next: [09. Integration tests](09-tests-routes.md) · [Index](../README.md)

`src/storage/log.rs` is the only file that touches the disk. It stores the log as a plain text file with one timestamp per line:

```
Mon, Oct 06 2026 14:03:09
Mon, Oct 06 2026 18:47:51
Tue, Oct 07 2026 08:12:30
```

It knows nothing about HTTP. Its functions take a file path and return `anyhow::Result`.

New concept in this doc:

- [Testing](../concepts/rust/testing.md) (the unit tests at the bottom of the file)

Already covered, and used heavily here: [pattern matching](../concepts/rust/pattern-matching.md) (the `match` guard, tuple destructuring), [`String` vs `&str` and lifetimes](../concepts/rust/ownership-and-strings.md), [anyhow](../concepts/rust/anyhow.md), [the builder pattern and `const`](../concepts/rust/structs-and-impl.md).

> **Blocking I/O:** these functions use `std::fs`, which blocks the thread, and they're called from async handlers. That breaks the "don't block in async" rule, on purpose: the files are tiny and there's one user. See [async and tokio](../concepts/rust/async-and-tokio.md#this-repos-choice).

---

## Imports and the constant

```rust
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
};

use anyhow::{Context, Result};
use chrono::Local;

const TIME_FORMAT: &str = "%a, %b %d %Y %H:%M:%S";
```

- `fs::{self, OpenOptions}`: `self` imports the `fs` module itself (for `fs::write` and `fs::read_to_string`), and `OpenOptions` imports the type.
- `io::{self, Write}`: `io` for `io::ErrorKind`. `Write` is the **trait** that gives `File` its `write_fmt` method, which `writeln!` calls. Without this import, `writeln!` fails to compile.
- `chrono::Local`: the local timezone (the server's own zone).
- `const TIME_FORMAT: &str`: a compile-time constant. `&str` here is really `&'static str`, a string stored inside the binary.

### The format string

| Code | Meaning | Example |
|------|---------|---------|
| `%a` | Short weekday name | `Mon` |
| `%b` | Short month name | `Oct` |
| `%d` | Day of month, 2 digits | `06` |
| `%Y` | 4-digit year | `2026` |
| `%H` | Hour 00–23 (24-hour clock) | `14` |
| `%M` | Minute | `03` |
| `%S` | Second | `09` |

Result: `Mon, Oct 06 2026 14:03:09`.

---

## `current_time`

```rust
/// Returns the current local time formatted as `Mon, Oct 06 2026 14:03:09`.
fn current_time() -> String {
    Local::now().format(TIME_FORMAT).to_string()
}
```

- It's **private** (there's no `pub`): only this module and its tests can call it.
- `Local::now()` gives a `DateTime<Local>`: the current time in the server's timezone. On Render that's UTC unless `TZ` is set (see [Config and deployment](../concepts/web/config-and-deploy.md)).
- `.format(TIME_FORMAT)` doesn't build a string yet. It returns a `DelayedFormat` value, which implements `Display`.
- `.to_string()` comes from the blanket `impl<T: Display> ToString for T`, and actually produces the `String`.

---

## `add_entry`

```rust
pub fn add_entry(file_name: &str) -> Result<String> {
    let entry = current_time();
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_name)
        .with_context(|| format!("failed to open {file_name}"))?;

    writeln!(file, "{entry}").with_context(|| format!("failed to write to {file_name}"))?;
    Ok(entry)
}
```

- `file_name: &str`: storage borrows the path. Handlers pass `&state.file_name`.
- `OpenOptions::new()...open(file_name)` uses the **builder pattern**:
  - `.append(true)`: every write goes to the **end** of the file. Existing lines are never overwritten.
  - `.create(true)`: create the file if it's missing. The first ever press creates the log.
  - `.open(...)` returns `io::Result<File>`.
- `let mut file`: writing changes the file handle's internal state, so the variable must be `mut`.
- `writeln!(file, "{entry}")` writes the entry plus `\n`. It returns `io::Result<()>`.
- `Ok(entry)` returns the text that was written, so the client sees exactly what got stored. The tests rely on this, so they never need to guess timestamps.
- The file is closed automatically when `file` goes out of scope at the end of the function. That's ownership's **drop** at work (RAII: "resource acquisition is initialization"). There's no `close()` call.

---

## `clear_all`

```rust
pub fn clear_all(file_name: &str) -> Result<()> {
    fs::write(file_name, "").with_context(|| format!("failed to clear the log in {file_name}"))?;
    Ok(())
}
```

`fs::write` creates the file, or **truncates** it (cuts it to zero length), then writes the contents, which here is nothing. So the file still exists, but it's empty. On a missing file, it creates an empty one.

---

## `show_log`

```rust
pub fn show_log(file_name: &str) -> Result<String> {
    match fs::read_to_string(file_name) {
        Ok(contents) => Ok(contents),
        // No log file yet means an empty log, not an error
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e).with_context(|| format!("failed to read {file_name}")),
    }
}
```

`fs::read_to_string` reads the whole file into a `String`. The `match` has three arms:

1. `Ok(contents)`: return it as it is.
2. `Err(e) if e.kind() == io::ErrorKind::NotFound`: a **match guard**. No file yet means nothing has been logged, so it's treated as an empty log. This is a design decision: before the first press, `GET /log` returns 200 with an empty body, not 500.
3. `Err(e)`: any other error (permission denied, the path is a directory, the content isn't UTF-8) is a real failure. `Err(e).with_context(...)` wraps the `io::Error` with a message and gives back an `anyhow::Result`.

The `match` is the whole function body, with no `;` after it, so its value is the return value. All arms produce `Result<String>`.

The guard arm must come **before** the general `Err(e)` arm. Arms are tried in order.

---

## `split_last_line`

```rust
/// Splits the log into (everything before the last line, the last line).
fn split_last_line(contents: &str) -> (&str, &str) {
    let trimmed = contents.trim_end_matches('\n');
    let start = trimmed.rfind('\n').map_or(0, |i| i + 1);
    trimmed.split_at(start)
}
```

A private helper used by both `show_last` and `clear_last`. Because they share it, they always agree on what "the last line" means.

- The return type `(&str, &str)` is a tuple of two slices that **borrow from `contents`**. Nothing is copied. The lifetime is elided; the full signature would be `fn split_last_line<'a>(contents: &'a str) -> (&'a str, &'a str)`.
- `trim_end_matches('\n')` removes **all** trailing newlines. `"a\nb\n"` becomes `"a\nb"`. That's needed because every entry ends with `\n`. Without trimming, the "last line" would be the empty string after the final newline.
- `rfind('\n')` finds the **last** newline's byte index, as an `Option<usize>`. `None` means there's only one line.
- `.map_or(0, |i| i + 1)`: the last line starts right after that newline, or at 0 when there's no newline.
- `split_at(start)` splits the string at that byte index into `(before, after)`.

Walking through `"first\nsecond\nthird\n"`:

| Step | Value |
|------|-------|
| `trimmed` | `"first\nsecond\nthird"` |
| `rfind('\n')` | `Some(12)` |
| `start` | `13` |
| `split_at(13)` | `("first\nsecond\n", "third")` |

| Input | `(rest, last)` |
|-------|----------------|
| `""` | `("", "")` |
| `"only\n"` | `("", "only")` |
| `"a\nb\n"` | `("a\n", "b")` |

`split_at` panics if the index isn't on a UTF-8 character boundary. Here it's always right after a `'\n'`, which is a single byte, or 0, so it's always a valid boundary.

---

## `show_last`

```rust
pub fn show_last(file_name: &str) -> Result<String> {
    let contents = show_log(file_name)?;
    let (_, last) = split_last_line(&contents);
    Ok(last.to_string())
}
```

- It reuses `show_log`, so a missing file means an empty log here too.
- `let (_, last)`: destructure the tuple and ignore the first part.
- `last.to_string()`: `last` borrows from `contents`, and `contents` is dropped when the function returns. So it has to be copied into an owned `String`. Returning `last` (a `&str`) directly would be a "does not live long enough" error.
- On an empty log this returns `""`, which the handler sends as 200 with an empty body.

---

## `clear_last`

```rust
pub fn clear_last(file_name: &str) -> Result<()> {
    let contents = show_log(file_name)?;
    let (rest, _) = split_last_line(&contents);

    fs::write(file_name, rest)
        .with_context(|| format!("failed to write {file_name} after clearing last line"))
}
```

- It reads everything, keeps `rest` (every line except the last, with their `\n`s), and writes that back.
- The last expression has **no `?` and no `Ok(())`**. `fs::write(...).with_context(...)` already has type `anyhow::Result<()>`, which is exactly the return type, so it's returned as it is.
- On an empty or missing log, `rest` is `""`, so it writes an empty file. "Does nothing" here means no entries get removed, though a missing file gets created.

**Known limitation:** this is a non-atomic read-then-write. If a `POST /log` lands between the read and the write, that new entry is lost. If the process crashes mid-write, the file can end up truncated. The plan is to fix this with the `LogStore` work, for example with a mutex.

---

## Unit tests

```rust
#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    ...
}
```

- `#[cfg(test)]`: this module only exists in `cargo test` builds.
- `mod tests` is an inline child module. It can see private items like `current_time` and `split_last_line`, because child modules can see their parent's private items.
- `use super::*;` imports everything from the parent module: the functions, plus the parent's own imports like `fs`.

See [Testing](../concepts/rust/testing.md).

### The helper

```rust
    #[track_caller]
    fn temp_log() -> (TempDir, String) {
        let dir = TempDir::new().expect("temp dir should be creatable");
        let path = dir
            .path()
            .join("test.log")
            .to_str()
            .expect("temp path should be valid utf-8")
            .to_string();
        (dir, path)
    }
```

- `TempDir::new()` creates a fresh, uniquely named folder under the system temp directory.
- It returns the `TempDir` **and** the path. The folder is deleted when the `TempDir` is dropped, so the caller must keep it alive: tests write `let (_dir, path)`, never `let (_, path)`.
- `.path().join("test.log")` gives a `PathBuf` for a file inside the folder. The file itself isn't created, so each test decides whether it exists.
- `.to_str()` gives `Option<&str>`: `None` if the path isn't valid UTF-8. The `.expect(...)` there is the planned target of the "store paths as `PathBuf`" change.
- `#[track_caller]`: if an `expect` in here panics, the panic message points at the **test** that called `temp_log()`, not at this helper.

### The tests

Each test follows **arrange / act / assert**, with blank lines between the three parts.

| Test | Arrange | Act | Assert |
|------|---------|-----|--------|
| `current_time_matches_log_format` | — | `current_time()` | Parses back with `NaiveDateTime::parse_from_str` and the format written out as a literal (not the constant). If someone changes `TIME_FORMAT`, this test notices. |
| `show_log_returns_empty_when_file_missing` | No file | `show_log` | `""`. This covers the `NotFound` guard. |
| `show_log_errors_when_path_is_unreadable` | A **directory** at the log path | `show_log` | `is_err()`. This covers the third arm: reading a directory fails with something other than `NotFound`. |
| `add_entry_appends_without_overwriting` | No file | `add_entry` twice | The file equals `"{first}\n{second}\n"`, using the returned entries, not guessed times. |
| `show_last_returns_final_line` | Seed `"first\nsecond\n"` | `show_last` | `"second"` |
| `show_last_is_empty_when_log_missing` | No file | `show_last` | `""` |
| `clear_last_keeps_earlier_lines` | Seed 3 lines | `clear_last` | The first 2 lines remain |
| `clear_last_on_single_line_leaves_empty_file` | Seed `"only\n"` | `clear_last` | `""` |
| `clear_all_empties_file` | Seed 2 lines | `clear_all` | `""` |

One example in full:

```rust
    #[test]
    fn clear_last_keeps_earlier_lines() {
        let (_dir, path) = temp_log();
        fs::write(&path, "first\nsecond\nthird\n").expect("log should be seedable");

        clear_last(&path).expect("last entry should be clearable");

        let contents = fs::read_to_string(&path).expect("log file should be readable");
        assert_eq!(contents, "first\nsecond\n");
    }
```

- Arrange: a temp path, seeded with known text.
- Act: one call. `expect` turns an unexpected `Err` into a test failure with a readable message.
- Assert: read the file back (part of the assert block), then `assert_eq!(actual, expected)`.

These 9 tests, together with the integration tests, leave `cargo mutants` with **zero missed mutants** in this file. Every branch and every `+ 1` is pinned down by some test.

---

## Try it

1. Change `map_or(0, |i| i + 1)` to `map_or(0, |i| i)` and run `cargo test`. Which tests fail, and what does the wrong output look like?
2. Remove `.append(true)`. What happens to the second write? Which test catches it?
3. Write a test for `split_last_line("a\n\n\n")`. What should it return?
4. Run `cargo mutants --file src/storage/log.rs` and read the summary.

## Quick revise

- Storage is a plain text file, one `%a, %b %d %Y %H:%M:%S` timestamp per line. It uses blocking `std::fs`, and knows nothing about HTTP.
- `use io::Write` is needed for `writeln!`. `OpenOptions::new().append(true).create(true).open(...)`. Files close on drop (RAII).
- `show_log`: a missing file (`NotFound` guard) is an empty log. Any other error gets context.
- `split_last_line`: `trim_end_matches('\n')`, then `rfind('\n').map_or(0, |i| i + 1)`, then `split_at`. It returns borrowed slices. `show_last` and `clear_last` share it.
- `show_last` returns `last.to_string()`, because the borrow can't outlive `contents`.
- `clear_last` returns `fs::write(...).with_context(...)` directly. It's a non-atomic read-then-write (a known race).
- Tests: `#[cfg(test)] mod tests` + `use super::*`. `temp_log()` returns `(TempDir, path)`, which you keep alive as `_dir`. Arrange/act/assert. Seed known text. Never assert exact times.
