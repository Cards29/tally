---
tags: [concept, rust]
aliases: ["ownership", "borrowing", "lifetimes", "String vs &str", "shadowing", "move", "clone"]
---

# Ownership, borrowing and strings

> Concept · First seen in: [02. main.rs and the module tree](../../files/02-main-and-modules.md) · Prev: [Modules and crates](modules.md) · Next: [Structs and `impl`](structs-and-impl.md) · [Index](../../README.md)

This is the idea that makes Rust different from other languages. Take it slowly. Everything else builds on it.

## The problem it solves

Every program has to free memory it no longer needs.

- **C** makes you do it by hand. Forget, and memory leaks. Free twice, or use memory after freeing it, and you get crashes and security holes.
- **Java, Python, Go and JavaScript** use a **garbage collector**: a background process that finds unused memory. It's safe, but it costs speed and makes pauses hard to predict.
- **Rust** decides *at compile time* exactly when each value gets freed, using three rules the compiler checks. There's no garbage collector and no manual freeing.

## The three ownership rules

1. Every value has exactly one **owner**: a variable, a field, and so on.
2. There can only be one owner at a time.
3. When the owner goes out of scope, the value is **dropped** (freed).

```rust
fn main() {
    let s = String::from("hi"); // s owns the String
    println!("{s}");
}                               // s goes out of scope here, so the String is freed
```

## Moves

Assigning a value or passing it to a function **moves** ownership to the new place. The old variable can't be used afterwards:

```rust
let a = String::from("hi");
let b = a;            // ownership moves from a to b
// println!("{a}");   // error[E0382]: borrow of moved value: `a`

fn take(s: String) {} // `take` now owns s, and drops it when it returns
take(b);
// take(b);           // error: b was already moved
```

Why a move and not a copy? A `String` holds a pointer to memory on the heap. If both `a` and `b` owned that memory, both would free it, which is a "double free" bug. Moving makes sure there's only ever one owner.

Moves work field by field. `main.rs` does this:

```rust
let app = routes::router(config.state); // moves the state field out
// config.port is still usable; config.state is not
```

### `Copy` types

Small, simple types are copied instead of moved: integers, `bool`, `char`, `f64`, and tuples made only of those. Copying them is as cheap as moving, and there's no heap memory to free twice.

```rust
let x: u16 = 3000;
let y = x;         // copy
println!("{x}");   // fine
```

### `.clone()`

If you really need two owned copies, ask for one explicitly:

```rust
let a = String::from("hi");
let b = a.clone(); // a deep copy; both are usable
```

`clone` can be expensive, and Rust makes you write it so you can see the cost. `routes.rs` calls `state.clone()` because both the middleware and the router need their own `AppState`.

## Borrowing: references

Often a function only needs to *look at* a value. Moving it in and then back out would be silly. Instead, you lend it a **reference** with `&`:

```rust
fn len(s: &String) -> usize { s.len() }

let s = String::from("hi");
let n = len(&s);  // borrow s
println!("{s}");  // s is still ours
```

### Mutable references

`&T` is a **shared** reference: read-only. `&mut T` is a **mutable** reference: it can change the value.

```rust
fn shout(s: &mut String) { s.push('!'); }

let mut s = String::from("hi"); // the variable itself must be `mut`
shout(&mut s);
```

### The borrowing rule

At any moment you can have **either**:

- any number of `&T`, **or**
- exactly one `&mut T`.

Never both at once. This one rule prevents data races at compile time. It also prevents a whole class of bugs like "I changed a list while looping over it".

```rust
let mut v = vec![1, 2, 3];
let first = &v[0];
// v.push(4);         // error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
println!("{first}");
```

Why is that an error? `push` might move the vector's contents to a bigger memory block. `first` would then point at freed memory.

## Lifetimes, briefly

A reference must never outlive the value it points to. The compiler checks this with **lifetimes**: labels for "how long this reference stays valid".

Most of the time you never write lifetimes, because the compiler fills them in (called **elision**). From `src/storage/log.rs`:

```rust
fn split_last_line(contents: &str) -> (&str, &str) { ... }
```

With the lifetimes written out, it would be:

```rust
fn split_last_line<'a>(contents: &'a str) -> (&'a str, &'a str) { ... }
```

This says: "the two returned slices point into `contents`, so they're valid only as long as `contents` is". The rule the compiler uses: if there's exactly one reference among the inputs, every reference in the output gets that same lifetime.

`'static` is a special lifetime meaning "valid for the whole program". String literals have the type `&'static str` because they're stored inside the compiled binary.

## `String` vs `&str`

Rust has two main string types. This confuses everyone at first.

| | `String` | `&str` (a "string slice") |
|-|----------|---------------------------|
| Owns its data? | Yes | No, it borrows |
| Where the data lives | On the heap | Anywhere: inside a `String`, or in the binary |
| Can grow? | Yes (`push_str`) | No |
| Typical use | Struct fields, return values, building text | Function parameters, string literals |

```rust
let lit: &str = "hello";               // a literal, stored in the binary
let owned: String = lit.to_string();   // copies it into a new heap String
let slice: &str = &owned;              // borrows the String as a &str
let part: &str = &owned[0..2];         // a slice of part of it: "he"
```

You can go back and forth:

- `&str` to `String`: `.to_string()`, `String::from(...)`, `.to_owned()`, or `format!(...)`.
- `String` to `&str`: `&s`, or `s.as_str()`. This is free; no copy is made.

**Rule of thumb:** take `&str` as a parameter, and store or return `String`. A `&str` parameter accepts both literals and borrowed `String`s:

```rust
pub fn show_log(file_name: &str) -> Result<String> { ... }

show_log("dates.log");          // a literal
show_log(&state.file_name);     // a String field, borrowed
```

`&String` automatically becomes `&str`. This is called **deref coercion**.

Strings are always valid UTF-8. `.as_bytes()` gives you the raw `&[u8]` bytes. The auth middleware compares the token's bytes.

## Shadowing

A new `let` with the same name hides the old variable:

```rust
let port = "3000";               // &str
let port: u16 = port.parse()?;   // a new variable, a new type, the same name
```

This is not mutation. These are two different variables, and the type can change. The repo's tests use it on purpose: `let (status, body) = ...` again for each step, instead of `status1`, `status2`.

## Common compiler errors

| Error | Meaning | Fix |
|-------|---------|-----|
| `E0382: borrow of moved value` | You used a value after moving it | Borrow it with `&`, `.clone()` it, or reorder the code |
| `E0502` / `E0499` | You broke the borrowing rule (`&` and `&mut` at once, or two `&mut`) | Shorten one borrow, or restructure |
| `E0106: missing lifetime specifier` | The compiler can't guess which input the output borrows from | Write `<'a>` explicitly |
| `E0597: does not live long enough` | A reference outlives its value | Return an owned value instead |
| `E0308: expected String, found &str` | Wrong string type | Add `.to_string()` or `&` |

## Try it

1. Write `let a = String::from("x"); let b = a; println!("{a}");` and read the error. Then fix it in two ways: with `&a`, and with `a.clone()`.
2. In a scratch function, write `fn first(a: &str, b: &str) -> &str { a }` and read error E0106. Add `<'a>` to make it compile.

## Read more

- [The Rust Book, ch. 4: Understanding Ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [The Rust Book, ch. 10.3: Lifetimes](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)

## Quick revise

> [!TIP]
> - Each value has one owner, and is dropped when the owner goes out of scope. There's no GC.
> - Assigning or passing a value **moves** it, and the old name becomes unusable. Small types are `Copy`. Use `.clone()` for an explicit deep copy.
> - `&T`: shared, read-only borrow. `&mut T`: exclusive borrow. Many `&` **or** one `&mut`, never both.
> - Lifetimes stop references from outliving their data. They're usually elided. `'static` means valid for the whole program.
> - `String` is owned and growable. `&str` is a borrowed view. Take `&str` as a parameter; store or return `String`. `&String` turns into `&str` automatically.
> - Shadowing: `let x` again makes a new variable, and its type can change.
