# Snakeway Overview

Snakeway is a programmable edge proxy built on top of **Pingora**.

Its goal is to provide **predictable extensibility** without sacrificing performance or correctness.

This document explains *how Snakeway thinks*.

## Design Goals

Snakeway is built around a few non-negotiable principles:

### 1. Explicit Execution

Every request follows a clearly defined lifecycle.  
There are no hidden hooks, background mutations, or implicit side effects.

```
on_request → before_proxy → after_proxy → on_response
```

If logic runs, it runs in one of these phases.

### 2. Devices, Not Scripts

Snakeway does not embed a general-purpose scripting language.

Instead, it uses **Devices**:

- Small, focused processing units
- Composable
- Ordered
- Explicitly scoped

Devices can:

- Inspect or modify requests
- Shape responses
- Block traffic
- Emit metrics
- Enrich request context (identity, geo, fraud signals)

Built-in devices are written in Rust.  
Third-party logic can be loaded via WASM (optional, sandboxed).

### 3. Request-Scoped Context

Each request carries a mutable, request-scoped context.

- No global state
- No thread-local hacks
- No implicit mutation

Devices communicate by extending this context in a controlled way.

## Hot Reload Model

Snakeway supports hot reload via:

- `SIGHUP`
- `POST /admin/reload`

On reload:

1. Configuration is parsed and validated
2. Routes are rebuilt
3. Device registry is reconstructed
4. New state is atomically swapped in

Existing requests complete normally.  
New requests see the updated configuration.

## Observability First

Snakeway treats observability as a first-class concern.

Out of the box:

- Structured logs
- Request counters
- Upstream latency tracking
- Error metrics

Metrics are designed to be consumed by external systems rather than hidden behind dashboards.

## What Snakeway Is Not

Snakeway intentionally avoids:

- Implicit global scripting
- Runtime code mutation
- Control-plane-heavy architectures
- “Magic” auto-configuration

It favors correctness, debuggability, and long-term maintainability over convenience.

## Who Snakeway Is For

Snakeway is built for:

- Infrastructure engineers
- Edge and ad-tech workloads
- Teams who want control without chaos
- Developers who value explicit systems

If nginx is a wrench, Snakeway is a machine shop.
