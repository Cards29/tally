# Pattern matching

> Concept · First seen in: [05. Auth middleware](../../files/05-middleware-auth.md) · Prev: [anyhow](anyhow.md) · Next: [Macros and attributes](macros-and-attributes.md) · [Index](../../README.md)

A **pattern** describes the shape of a value, and can pull pieces out of it. Patterns show up in many more places than `match`: `let`, function parameters, `for` loops, `if let`, and closures.

## `match`

```rust
let n = 3;
let word = match n {
    0 => "zero",
    1 | 2 => "small",         // `|` means "or"
    3..=9 => "single digit",  // an inclusive range
    _ => "big",               // `_` matches anything (the catch-all)
};
```

- Arms are tried **top to bottom**, and the first match wins.
- `match` is an **expression**: it produces a value. All arms must produce the same type.
- `match` must be **exhaustive**: every possible value must be covered by some arm. Forget `None` and the program won't compile. That's what makes `Option` and `Result` safe.

### Matching enums and pulling out data

```rust
match fs::read_to_string(path) {
    Ok(contents) => println!("{contents}"),    // binds the inner value to `contents`
    Err(e) => eprintln!("{e}"),
}
```

### Guards

An `if` after the pattern adds an extra condition. From `src/storage/log.rs`:

```rust
match fs::read_to_string(file_name) {
    Ok(contents) => Ok(contents),
    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
    Err(e) => Err(e).with_context(|| format!("failed to read {file_name}")),
}
```

The second arm only matches errors whose kind is `NotFound`. Every other error falls through to the third arm. Order matters: put the specific arm before the general one. If the arms were swapped, the plain `Err(e)` would catch everything, and the compiler would warn that the guarded arm is unreachable.

## Destructuring

Patterns can take apart tuples, structs and enums anywhere a `let` would go.

### Tuples

```rust
let (rest, last) = split_last_line(&contents);   // the function returns (&str, &str)
let (_, last) = split_last_line(&contents);      // ignore the first part
let (status, body) = send(...).await;            // the tests use this all the time
```

### Structs and tuple structs

```rust
struct Point { x: i32, y: i32 }
let Point { x, y } = p;              // two new variables: x and y
let Point { x, .. } = p;             // `..` ignores the other fields

struct State<S>(pub S);              // roughly how axum defines State
let State(inner) = state_extractor;  // unwrap the single field
```

### In function parameters

A parameter is a pattern too:

```rust
async fn show_log(State(state): State<AppState>) -> ... { /* use `state` directly */ }
fn dist((x, y): (f64, f64)) -> f64 { (x * x + y * y).sqrt() }
```

### In `for` loops

```rust
for (method, uri) in PROTECTED {    // each item is a (Method, &str) tuple
    ...
}
```

### In closures

```rust
pairs.iter().map(|(a, b)| a + b)
```

## `_` vs `_name`: an important difference

| Pattern | Binds? | When is the value dropped? |
|---------|--------|----------------------------|
| `_` | **No** | Right away, at the end of the statement, if it was a temporary |
| `_dir` | Yes (the underscore just silences the "unused" warning) | At the end of the scope, like any variable |

This matters for `TempDir`, which deletes its folder when dropped:

```rust
let (_dir, path) = temp_log(); // correct: the folder lives until the test ends
let (_, path) = temp_log();    // bug: the TempDir is dropped right away and the folder is deleted
```

## `if let` and `let ... else`

When you only care about one case:

```rust
if let Some(token) = token {
    request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
}
```

`let ... else` binds the value or leaves the function:

```rust
let Some(token) = token else {
    return StatusCode::UNAUTHORIZED.into_response(); // the else block must exit
};
// token is a plain &str from here on
```

## `matches!`

This gives a `bool` without writing a whole `match`:

```rust
if matches!(e.kind(), io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied) { ... }
```

## Refutable vs irrefutable

- **Irrefutable** patterns always match: `x`, `(a, b)`, `State(s)`. These are allowed in `let`, function parameters and `for`.
- **Refutable** patterns might not match: `Some(x)`, `Ok(v)`. These need `match`, `if let` or `let ... else`.

`let Some(x) = opt;` without an `else` is a compile error (E0005), because the pattern might not match.

## Common compiler errors

| Error | Meaning |
|-------|---------|
| `E0004: non-exhaustive patterns: None not covered` | Add the missing arm, or a `_` arm |
| `E0005: refutable pattern in local binding` | Use `if let` or `let ... else` |
| `unreachable pattern` (a warning) | An earlier arm already covers this one |
| `E0308: mismatched types` in match arms | All arms must return the same type |

## Try it

1. Remove the `Err(e) if ...NotFound` arm from `show_log` and run `cargo test`. Which test fails?
2. Swap the two `Err` arms and read the warning.
3. Change `let (_dir, path)` to `let (_, path)` in one storage test, and see whether it still passes. Why? (Hint: does the test write a file before reading?)

## Read more

- [The Rust Book, ch. 6.2: `match`](https://doc.rust-lang.org/book/ch06-02-match.html)
- [The Rust Book, ch. 19: Patterns and Matching](https://doc.rust-lang.org/book/ch19-00-patterns.html)

## Quick revise

- `match` is an expression. Arms run top to bottom. It must be exhaustive. Use `|` for "or", `a..=b` for ranges, `_` as the catch-all.
- Guards: `Err(e) if cond =>`. Put specific arms before general ones.
- Destructure in `let`, function parameters, `for`, and closures: `(a, b)`, `Point { x, .. }`, `State(state)`.
- `_` drops right away. `_name` lives until the end of the scope. Keep guards like `TempDir` in `_dir`.
- `if let Some(x) = ...` handles one case. `let Some(x) = ... else { return ... };` binds or exits. `matches!` gives a bool.
- Irrefutable patterns go in `let` and parameters. Refutable ones need `match`, `if let`, or `let ... else`.
