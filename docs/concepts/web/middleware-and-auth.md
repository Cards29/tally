# Middleware and auth

> Concept · First seen in: [04. Routes](../../files/04-routes.md), used in [05. Auth middleware](../../files/05-middleware-auth.md) · Prev: [axum](axum.md) · Next: [Config and deployment](config-and-deploy.md) · [Index](../../README.md)

## Middleware

**Middleware** is code that runs *around* handlers. It sees each request before the handler, and each response after. Typical uses:

- authentication
- logging and timing
- compression
- CORS
- rate limiting
- timeouts

The usual picture is an onion:

```
request ─► [ auth ─► [ logging ─► handler ] ] ─► response
                 │
                 └── can stop early: return 401 without calling the inner layers
```

Each layer can:

1. look at or change the request
2. **either** pass it inward (`next.run(request).await`) **or** return its own response right away
3. look at or change the response on the way out

### tower: `Service` and `Layer`

axum uses tower's model:

- A **`Service`** is "something that takes a request and returns a future of a response". Handlers and routers are services.
- A **`Layer`** wraps one service and produces a new service around it.

Writing raw tower layers is verbose. axum gives you a shortcut:

```rust
use axum::middleware::{from_fn, from_fn_with_state, Next};

async fn my_middleware(request: Request, next: Next) -> Response {
    // before the handler
    let response = next.run(request).await;
    // after the handler
    response
}

router.layer(from_fn(my_middleware));
router.layer(from_fn_with_state(state.clone(), my_middleware_with_state));
```

The middleware function's arguments are extractors, just like a handler's: `State<S>` and others first, then `Request`, then `Next` last. `Next` represents "the rest of the onion".

### `layer` vs `route_layer`

| Method | Wraps | Requests to unknown paths |
|--------|-------|---------------------------|
| `.layer(l)` | All routes added so far, **and** the fallback | Also go through the middleware |
| `.route_layer(l)` | Only routes that matched | Skip it, and get a plain 404 |

For auth, `route_layer` is the right choice. Auth should guard real routes, not turn every typo into a 401.

Either way, a layer only wraps routes added **before** it on the same router.

## Authentication vs authorization

- **Authentication**: *who* are you? (passwords, tokens, sessions)
- **Authorization**: *what* are you allowed to do? (roles, permissions)

Tally only does authentication, with one shared secret. Whoever has the token can do everything.

## Bearer tokens

The client sends:

```
Authorization: Bearer <token>
```

"Bearer" means *whoever bears (holds) this token gets in*. There's no further identity check. So:

- **Always use HTTPS.** Over plain HTTP, the token is readable by anyone on the network path. Render provides HTTPS.
- Make the token long and random.
- Never put it in a URL (`?token=...`). URLs get logged.
- If it leaks, change it (in Render's dashboard), and restart.

Bigger systems use other schemes: JWTs (signed tokens that carry claims), OAuth, or session cookies. They all solve problems Tally doesn't have, like many users and expiry.

## Timing attacks

A naive check:

```rust
if provided == secret { ... }   // == on strings stops at the first differing byte
```

`==` returns as soon as one byte differs. A wrong token that matches the first 5 characters takes slightly longer to reject than one that matches none. With enough requests and statistics, an attacker can measure that difference and guess the token **one byte at a time**. This is a **timing side channel**.

The fix is a **constant-time comparison**: always check every byte, and combine the results without branching:

```rust
use subtle::ConstantTimeEq;

let ok: bool = bool::from(provided.as_bytes().ct_eq(secret.as_bytes()));
```

- `ct_eq` comes from the `subtle` crate's `ConstantTimeEq` trait. It's implemented for byte slices `[u8]`.
- It returns a `subtle::Choice`, not a `bool`. A `Choice` is a wrapper meant to stop the compiler from optimizing the comparison back into an early exit. You convert it with `bool::from(...)`, or `.into()`, only at the end.
- Slices of different **lengths** are rejected right away, so the token's length can leak. The length isn't secret enough to matter here.

Over the internet, network noise makes this attack hard. But constant-time comparison costs nothing, so always use it for secrets.

## Read more

- [axum::middleware docs](https://docs.rs/axum/latest/axum/middleware/index.html)
- [subtle docs](https://docs.rs/subtle)
- [OWASP: Authentication cheat sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)

## Quick revise

- Middleware wraps handlers like an onion. It can change the request, stop early with its own response, or change the response.
- `from_fn_with_state(state, f)`, where `f(State, ..., Request, Next) -> Response`. Call `next.run(request).await` to continue inward.
- `route_layer` runs only for matched routes, so unknown paths still get 404. `layer` also wraps the fallback. Both only wrap routes added before them.
- Authentication = who. Authorization = what they may do. Tally only does authentication.
- Bearer token: whoever holds it gets in. HTTPS only, long and random, never in URLs.
- `==` leaks timing. Use `subtle`'s `ct_eq`, which returns a `Choice`, then `bool::from` it.
