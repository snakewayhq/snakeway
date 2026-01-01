# Builtin Devices

Builtin devices are first‑class, in‑process extensions that run directly inside the Snakeway request pipeline. They are
designed for **high‑performance, low‑latency** behavior where sandboxing is unnecessary and tight integration with
Snakeway internals is beneficial.

If you are familiar with Laravel, you can think of builtin devices as a blend of **middleware** and **service providers
**: they participate in the request lifecycle, but they also establish shared capabilities that other devices can rely
on.

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

Builtin devices are declared under the `[[device]]` section:

```toml
[[device]]
name = "identity"
type = "builtin"
builtin = "identity"
enabled = true

[device.config]
enable_geoip = true
enable_user_agent = true
```

Key fields:

| Field             | Description                               |
|-------------------|-------------------------------------------|
| `name`            | Logical device name (for logs and errors) |
| `kind`            | Must be `"builtin"`                       |
| `builtin`         | Builtin device identifier                 |
| `enabled`         | Whether the device is active              |
| `devices.options` | Device‑specific configuration             |

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

## Design Philosophy

Builtin devices follow a few core principles:

### 1. Single Source of Truth

If data can be derived once and reused, it should be.

Example: client IP resolution should happen once, not in every device.

### 2. Internal by Default

Builtin devices should **not** mutate headers or leak data unless explicitly configured to do so.

### 3. Composition Over Coupling

Devices communicate via typed context extensions, not direct references.

This keeps devices independent while still cooperating.

### 4. Explicit Risk Boundaries

Anything with privacy, security, or compliance impact must be:

* Opt‑in
* Narrowly scoped
* Easy to audit

## Available Builtin Devices

See the following pages for concrete implementations:

* **Identity** — Canonical client IP, proxy chain, geo, and UA enrichment
* **Structured Logging** — Lifecycle‑aware structured logs

More builtin devices will be added over time as Snakeway evolves.

## When to Write a Builtin Device

Write a builtin device if:

* Performance is critical
* You need access to internal request context
* The behavior is foundational or infrastructural

Prefer WASM devices if:

* You want isolation
* You expect third‑party or user‑supplied code
* The logic is business‑specific
