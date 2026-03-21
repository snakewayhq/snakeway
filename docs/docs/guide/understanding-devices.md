---
title: Understanding Devices
---

Devices are composable units of logic that execute inside the Snakeway request pipeline. Each device can observe, mutate, enrich, or short-circuit traffic at well-defined points in the request/response lifecycle. The device pipeline is the primary extension mechanism in Snakeway, and understanding how devices compose is essential for configuring and operating the proxy effectively.

## Builtin vs WASM Devices

Snakeway supports two categories of devices.

**Builtin devices** are Rust implementations compiled directly into the Snakeway binary. They run in-process with no sandboxing overhead, have full access to Rust APIs, and share memory with the core request context. Builtin devices are intended for core infrastructure concerns: identity resolution, logging, rate limiting, and network policy enforcement.

**WASM devices** run inside a sandboxed WebAssembly runtime. They communicate with Snakeway through a defined host API and cannot access process memory directly. WASM devices are appropriate for business-specific policy logic, especially when the code comes from third parties or needs strong isolation guarantees.

| Property | Builtin | WASM |
|---|---|---|
| Execution | In-process | Sandboxed VM |
| Performance | Maximum | Slight overhead |
| API access | Full Rust | Limited host API |
| Trust model | Trusted code only | Safe for untrusted code |

A common deployment pattern uses builtin devices for core primitives (Identity, Structured Logging, Rate Limiting) and WASM devices for application-specific routing or transformation logic.

## The Device Pipeline

Devices execute in a defined order on every request. To illustrate how they compose, consider a request flowing through a pipeline with Identity, Network Policy, Rate Limiting, and Structured Logging enabled:

1. **Identity** resolves the client IP from `X-Forwarded-For` and trusted proxies, performs GeoIP lookup, and parses the user agent. It stores the result as a `ClientIdentity` extension.
2. **Network Policy** reads the `ClientIdentity` extension and checks the resolved IP against its CIDR allowlist. If the IP is disallowed, the request is rejected with `403 Forbidden` immediately.
3. **Rate Limiting** reads the `ClientIdentity` extension and checks whether the client has exceeded its request rate. If so, the request is rejected with `429 Too Many Requests`.
4. **Structured Logging** emits a tracing event containing the HTTP method, URI, and selected identity fields.

If any device returns a response (such as a 403 or 429), the pipeline halts and no further devices or upstream proxying occurs. This short-circuit behavior is the mechanism that makes security enforcement efficient.

## Lifecycle Hooks

Each device can hook into one or more phases of the request/response lifecycle. The phases execute in this order for proxied requests:

`on_request`, `on_stream_request_body` (zero or more times), `before_proxy`, `after_proxy`, `on_response`

For static file routes, only `on_request` and `on_response` execute. Proxy-specific phases are skipped entirely.

| Hook | Fires when | Typical use |
|---|---|---|
| `on_request` | A request arrives | Inspection, mutation, early deny decisions, identity resolution |
| `on_stream_request_body` | A body chunk is received | Body inspection, size enforcement |
| `before_proxy` | Just before upstream connection | Header injection, routing overrides |
| `after_proxy` | Upstream response received | Response header modification, error recording |
| `on_response` | Response is about to be sent | Logging, metrics, auditing |

For a detailed description of each phase, including capability constraints and static route behavior, see the [Lifecycle internals](/docs/internals/lifecycle) page.

## Device Ordering and Dependencies

Some devices depend on data produced by others. The Identity device must run before Network Policy and Rate Limiting, because both consume the `ClientIdentity` extension that Identity creates. Snakeway enforces this ordering automatically based on declared dependencies.

When configuring devices, you do not need to specify execution order manually. Snakeway resolves the dependency graph at startup and arranges the pipeline accordingly.

## The Extension Mechanism

Devices communicate through a typed extension store on the request context. A device can insert a strongly-typed value into the store, and any downstream device can retrieve it by type.

Extensions are request-scoped (they exist only for the duration of a single request), never forwarded upstream, and never included in logs unless a device explicitly opts in. This design ensures that devices can share computed data efficiently while maintaining clear boundaries between pipeline stages.
