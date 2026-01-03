# Builtin Devices

Builtin devices are first‑class, in‑process extensions that run directly inside the Snakeway request pipeline. They are
designed for **high‑performance, low‑latency** behavior where sandboxing is unnecessary and tight integration with
Snakeway internals is beneficial.

## What Are Builtin Devices?

A builtin device is a Rust implementation of the `Device` trait that is compiled into Snakeway itself.

Unlike WASM devices:

* They are **not sandboxed**
* They can access full Rust APIs
* They can share memory with the core request context
* They are extremely fast (no FFI or VM boundary)

Builtin devices are intended for:

* Identity and enrichment (IP, geo, UA)
* Logging and observability
* Security primitives (rate limiting, allow/deny lists)
* Core infrastructure features

## Execution Model

Builtin devices execute synchronously as part of the request lifecycle.

Each device may hook into one or more lifecycle phases:

* `on_request`
* `before_proxy`
* `after_proxy`
* `on_response`
* `on_error`

Devices are executed **in the order they are declared** in configuration.

> **Important**
> Devices earlier in the list may enrich or modify request context for devices that run later.

## Request Context & Extensions

Builtin devices operate on a shared `RequestCtx` and `ResponseCtx`.

In addition to headers and routing information, Snakeway provides a **typed extensions store**:

```rust
ctx.extensions.insert(MyType { ... });
```

This allows builtin devices to:

* Compute expensive data once
* Store it in a canonical form
* Expose it to downstream devices without re‑parsing headers

Extensions are:

* Request‑scoped
* Strongly typed
* Never forwarded upstream
* Never logged unless explicitly opted in

This pattern is central to how builtin devices cooperate.

## Configuration

The builtin identity device basic configuration:

```toml
[identity_device]
enable = true

enable_geoip = true
enable_user_agent = true
```

Key fields:

| Field               | Description                                                            |
|---------------------|------------------------------------------------------------------------|
| `enable`            | Whether the device is active                                           |
| `enable_geoip`      | Parse the client IP address to determine location                      |
| `enable_user_agent` | Parse the client user agent to determine browser and device attributes |

Unknown options are rejected to prevent silent misconfiguration.

## Builtin vs WASM Devices

| Builtin             | WASM                    |
|---------------------|-------------------------|
| Runs in‑process     | Runs in sandbox         |
| Maximum performance | Strong isolation        |
| Full Rust access    | Limited host API        |
| Trusted code only   | Safe for untrusted code |

A common pattern is:

* **Builtin devices** provide core primitives (identity, logging, metrics)
* **WASM devices** implement business‑specific policy on top
