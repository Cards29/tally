---
tags: [concept, web]
aliases: ["HTTP", "status codes", "methods", "headers", "REST"]
---

# HTTP basics

> Concept · First seen in: [04. Routes](../../files/04-routes.md) · Prev: [Testing](../rust/testing.md) · Next: [axum](axum.md) · [Index](../../README.md)

HTTP is the text protocol that browsers, phones and servers use to talk. A client sends a **request**, and the server sends back exactly one **response**.

## A request

```
POST /log HTTP/1.1                       <- method, path, version
Host: tally.onrender.com                 <- headers: one per line, "Name: value"
Authorization: Bearer abc123
Content-Length: 0
                                         <- a blank line ends the headers
(body: empty here)
```

## A response

```
HTTP/1.1 200 OK                          <- version, status code, reason
content-type: text/plain; charset=utf-8
content-length: 25

Tue, Oct 07 2026 14:03:09                <- body
```

That's all HTTP is: text in a fixed format, sent over a TCP connection. HTTPS is the same thing, encrypted with TLS.

## Methods

The method says what the client wants to do with the resource named by the path.

| Method | Meaning | Safe? | Idempotent? | In this repo |
|--------|---------|-------|-------------|--------------|
| GET | Read | yes | yes | `/`, `/health`, `/log`, `/log/last` |
| POST | Create, or "do something" | no | no | `/log` adds an entry |
| PUT | Replace entirely | no | yes | — |
| PATCH | Change part of it | no | no | — |
| DELETE | Remove | no | yes | `/log`, `/log/last` |

- **Safe**: doesn't change anything on the server. Crawlers and prefetchers may send GETs freely, so a GET must **never** change data.
- **Idempotent**: sending it twice has the same effect as sending it once. Clients and proxies may retry idempotent requests on network errors.

Note: by that definition, `DELETE /log/last` is *not* truly idempotent. Sending it twice removes two entries. That's fine for a personal app, but worth knowing.

## Paths and resources (REST style)

The **path** names a thing (a resource). The **method** is the action:

```
GET    /log         read the log
POST   /log         add to the log
DELETE /log         clear the log
GET    /log/last    read the last entry
DELETE /log/last    remove the last entry
```

Compare this with `POST /clearLastEntry`. REST style uses fewer, predictable URLs.

After the path there can be a **query string**: `/log?limit=10`. This app doesn't use one.

## Status codes

The first digit gives the class:

| Class | Meaning |
|-------|---------|
| 1xx | Informational (rare) |
| 2xx | Success |
| 3xx | Redirect: look somewhere else |
| 4xx | **Client** error: the request was wrong |
| 5xx | **Server** error: the server failed |

The codes this app uses (or plans to):

| Code | Name | When here |
|------|------|-----------|
| 200 | OK | Success, with a body |
| 204 | No Content | Success, with no body. The DELETE routes. |
| 307 | Temporary Redirect | `GET /` goes to `/health` |
| 401 | Unauthorized | The token is missing or wrong |
| 404 | Not Found | No such route. Planned for `GET /log/last` on an empty log. |
| 405 | Method Not Allowed | The path exists but not with this method, e.g. `PUT /log` |
| 500 | Internal Server Error | Storage failed. Details only go in the server log. |

Other common ones: **201 Created** (POST made a new resource), **400 Bad Request** (malformed input), **403 Forbidden** (you're known, but not allowed), **422** (input understood but invalid), **503** (temporarily unavailable).

**401 vs 403:** 401 means "who are you? Authenticate". 403 means "I know who you are, and the answer is no". This app only has one user, so 401 is enough.

### Redirects

A 3xx response carries a `Location` header. The client then requests that URL.

| Code | Permanent? | Keeps method and body? |
|------|------------|------------------------|
| 301 | yes | not guaranteed (may become GET) |
| 302 | no | not guaranteed |
| 307 | no | yes |
| 308 | yes | yes |

Browsers may cache permanent redirects forever, so use temporary ones unless you're sure.

## Headers

Headers are `Name: value` metadata. Header names are case-insensitive.

| Header | Direction | Purpose |
|--------|-----------|---------|
| `Authorization: Bearer <token>` | request | Credentials |
| `Content-Type` | both | The format of the body (`text/plain`, `application/json`) |
| `Content-Length` | both | The body size in bytes |
| `Location` | response | The redirect target |
| `Accept` | request | Which formats the client wants back |

## Bodies

The body is optional, and can be any bytes. This app sends plain text: a `String` body becomes `text/plain; charset=utf-8`. JSON is the most common format in real APIs. This app hasn't needed it yet.

## Trying it with curl

```sh
curl -i localhost:3000/health                                  # -i: show status and headers
curl -i -X POST -H "Authorization: Bearer $TOKEN" localhost:3000/log
curl -i -H "Authorization: Bearer $TOKEN" localhost:3000/log
curl -i -X DELETE -H "Authorization: Bearer $TOKEN" localhost:3000/log/last
curl -iL localhost:3000/                                       # -L: follow redirects
```

## Read more

- [MDN: An overview of HTTP](https://developer.mozilla.org/en-US/docs/Web/HTTP/Overview)
- [MDN: HTTP response status codes](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status)

## Quick revise

> [!TIP]
> - Request: method + path + headers + optional body. Response: status + headers + optional body. It's text over TCP. HTTPS adds TLS.
> - GET reads (safe, so it must never change data). POST creates or acts. DELETE removes. PUT and DELETE are meant to be idempotent.
> - REST: the path is the resource, the method is the action.
> - 2xx success (200, 204). 3xx redirect (307 is temporary and keeps the method). 4xx client's fault (401, 404, 405). 5xx server's fault (500).
> - 401 = not authenticated. 403 = authenticated but not allowed.
> - `Authorization: Bearer <token>` carries the credential. `Location` carries the redirect target.
