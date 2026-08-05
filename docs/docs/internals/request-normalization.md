---
title: Request Normalization
---

When a request reaches Snakeway, its target and headers arrive in whatever shape the client sent.
Before routing the request or running any device, Snakeway rewrites that input into a canonical form.
Everything after hydration, from route matching to device hooks to the upstream request line, observes the canonical form rather than the raw input.

## When normalization runs

Normalization happens once per request, when the request context is hydrated from the incoming session.
It runs before route matching and before the first device hook.

Each part of the request resolves to one of three outcomes.
An already canonical part is accepted.
A part that can be repaired, such as a path with a repeated slash, is rewritten, and the request continues with the repaired form.
A part that cannot be repaired is rejected, and Snakeway responds with `400 Bad Request` without contacting an upstream.

## Path

The path is normalized per RFC 3986 and RFC 9110:

- A missing leading slash is added, so a relative path becomes absolute.
- Dot segments (`.` and `..`) are resolved. A `..` that would climb above the root rejects the request.
- Consecutive slashes collapse to one.
- A trailing slash is removed, except on the root path `/`.
- A NUL byte rejects the request.

Percent-encoded sequences in the path are preserved as received.
Snakeway does not yet decode them, so route prefixes match the encoded form of the path.

## Query

The query is validated at hydration, and the original form is what travels upstream:

- A NUL byte rejects the request.
- An incomplete or invalid percent triplet rejects the request.
- Parameters are also decoded and sorted into a canonical list for internal use.
- The upstream receives the query exactly as the client sent it, including parameter order and `+` encoding.

A bare delimiter with no query, as in `GET /api?`, is canonicalized away.
The upstream receives the bare path with no trailing `?`.

## Headers

Header normalization is protocol specific.
HTTP/1.1 requests follow RFC 9110 and RFC 9112, and HTTP/2 requests follow RFC 9110 and RFC 9113.

- Header names are lowercased.
- Duplicate headers fold into a single comma separated value.
- Standard hop-by-hop headers, and any header named in the `Connection` header, are stripped and never forwarded.
- A request carrying both `Transfer-Encoding` and `Content-Length` is rejected, because the two headers disagree about where the message ends.
- Duplicate `Content-Length` headers with differing values are rejected for the same reason.
- A header value with invalid encoding rejects the request.

## What the upstream receives

The upstream request line is rebuilt from the canonical path and the original query.
For example, `GET //api/./items?b=2&a=1 HTTP/1.1` reaches the upstream as `GET /api/items?b=2&a=1 HTTP/1.1`.

## Rejection behavior

A rejected request receives `400 Bad Request` and never reaches routing or an upstream.
Requests judged to be smuggling attempts follow the same path.
See [Protocol Negotiation](protocol-negotiation.md) for how these responses relate to version and upgrade handling.

## Where the code lives

The normalization pass lives in `crates/snakeway-engine/src/execution/ctx/request/normalization/`, with one module per concern (`path.rs`, `query.rs`, `headers.rs`).
The replay tests under `crates/snakeway-tests/tests/http_replay/` assert the upstream-received request line for the path and query behaviors above.
