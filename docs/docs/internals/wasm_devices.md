---
title: WASM Device Internals
---

This page describes the internal architecture of the WASM device subsystem. For the
operator-facing guide, see [Authoring WASM Devices](../extension/authoring-wasm-devices.md).

## Runtime Stack

WASM devices run on [Wasmtime](https://wasmtime.dev/) using the **component model**. The
host-guest contract is defined in WIT (`crates/snakeway-wit/wit/`) at version
`snakeway:device@0.4.0`.

| Layer            | Role                                                      |
|------------------|-----------------------------------------------------------|
| `snakeway-wit`   | WIT interface definition (types, host, policy)            |
| `wasmtime`       | Compilation, instantiation, execution                     |
| `wasm_device.rs` | `Device` trait implementation, per-hook dispatch          |
| `engine.rs`      | Shared `Engine` with pooling allocator and epoch ticker   |
| `state.rs`       | `HostState` implementing the host interface               |
| `lifecycle.rs`   | Request/response snapshot building and result application |

## Engine and Pooling

All WASM devices share a single `wasmtime::Engine` created by `WasmEngine::new()`. The
engine is configured with:

- **Component model** enabled
- **Epoch interruption** for timeout enforcement
- **Pooling allocator** (`PoolingAllocationConfig`) with pre-allocated slots for instances,
  memories, and tables

The pooling allocator avoids per-instantiation memory allocation. Instance slots are reused
across hook calls, making instantiation O(1) after the initial pool setup.

An epoch ticker thread increments the engine epoch every 10ms. Hook deadlines are expressed
as epoch tick counts derived from `timeout_ms / EPOCH_TICK_MS`.

The `WasmEngine` struct owns the shutdown flag for the ticker thread. When the
`DeviceRegistry` is dropped (on shutdown or hot reload), the ticker thread exits.

## Device Loading

At startup, the `DeviceRegistry` collects all enabled WASM device configs and loads them
together:

1. Create a shared `WasmEngine`.
2. For each device config, compile the `.wasm` file into a `Component`.
3. Build a `Linker` and pre-instantiate via `DevicePre`. This amortizes type-checking and
   linking so only instantiation remains per-hook.
4. Store each `WasmDevice` (which holds an `Arc<Engine>` and the `DevicePre`) in the
   device pipeline.

## Per-Hook Execution

Before any of this, each `Device` hook method checks the device's hook mask
(`WasmHookMask`, lowered from the `hooks` config allowlist). If the hook is not enabled,
the method returns `DeviceResult::Continue` immediately, so no `Store` is created and no
instance is instantiated. This is the cheapest path: a device that declares only
`on_request` skips the store-setup, instantiate, snapshot, and apply cost for the other
five hooks. When `hooks` is absent, the mask enables all six (the original behavior).

For an enabled hook, each lifecycle call follows this sequence in
`WasmDevice::with_instance`:

1. Create a `StoreLimitsBuilder` with memory and table caps.
2. Build `HostState` with the device's config map, name, and metrics handles.
3. Create a `Store` on the shared engine with the host state and resource limits.
4. Set the epoch deadline: `timeout_ms / EPOCH_TICK_MS` ticks.
5. Instantiate via `DevicePre::instantiate` (uses a pooled memory slot).
6. Call the exported hook function.
7. Record hook duration in the `hook_duration_ms` histogram.
8. On error, route through `handle_failure` which respects the configured fail policy.

Instances are not reused across hooks. Each hook gets a clean sandbox with no residual
state from prior calls.

## Fail Policy

Every failure path (instantiation error, execution trap, timeout, body buffer overflow)
routes through `handle_failure`:

- **Open:** log at warn level, increment the failure counter, return `DeviceResult::Continue`.
- **Closed:** log at error level, increment the failure counter, return
  `DeviceResult::Respond` with `503 Service Unavailable`.

## Host State

The `HostState` struct implements the `host` interface:

- **`config-get`** reads from an `Arc<HashMap<String, String>>` populated from the HCL
  `config` block.
- **`log`** emits a `tracing` event at the mapped level, tagged with the device name.
- **`metric-increment`** adds to an OTel counter with `device` and `metric` labels.
- **`epoch-secs`** returns the current Unix timestamp.

## Patch Application

WASM devices never mutate the request or response directly.
Each hook receives a read-only snapshot and returns a **result** that carries an **action** and an optional **patch**.
The host interface exposes no function that mutates in-flight traffic, so the only way a device changes a request or
response is by returning a patch for the host to apply.

### Staging context, not the live message

The host applies each patch to a staging context, `RequestCtx` or `ResponseCtx`, which is Snakeway's own normalized copy
of the request or response.
This staging context is a separate object from the Pingora `RequestHeader` and `ResponseHeader` that actually travel to
the upstream and back to the client.

Two functions in `lifecycle.rs` apply patches to the staging context:

- `apply_request_result` handles `Action::Continue` (with an optional `RequestPatch`), `Action::Block` (403), and
  `Action::Respond` (a synthetic response with custom status, headers, and body). A `RequestPatch` can rewrite the route
  path, override the upstream path, and mutate headers.
- `apply_response_result` handles the same actions for response hooks, including status overrides and header mutations.

Header operations (`HeaderOp::Set`, `Append`, `Remove`) are validated before application.
An invalid header name or value is logged at warn level and skipped.

### Writeback to the Pingora message

Because the staging context is separate from the message on the wire, the host reconciles it back into the Pingora
`RequestHeader` or `ResponseHeader` at the correct Pingora hook.
For headers and status this uses a clear-and-repopulate strategy.

- **Request path.** `on_request` runs in Pingora's `request_filter` and `before_proxy` runs in
  `upstream_request_filter`. Both apply their patch to `RequestCtx`. In `upstream_request_filter` the host sets the
  upstream method and path from the context, then clears the upstream request headers and repopulates them from
  `ctx.headers()`.
- **Response path.** `after_proxy` runs in `upstream_response_filter` and `on_response` runs in `response_filter`. Both
  apply their patch to `ResponseCtx`. The host then sets the upstream status and clears and repopulates the upstream
  response headers from the context.

:::caution
The writeback clears the headers first and then repopulates them, rather than inserting over the originals.
An insert-only writeback would not carry a `HeaderOp::Remove` through to the wire, and would collapse an appended
multi-value header down to a single value.
Clearing first makes the forwarded headers exactly match the staging context, so `Set`, `Append`, and `Remove` all take
effect.
:::

### Bodies apply to the live buffer

Body patches follow a different strategy.
`apply_body_result` handles `BodyAction::Passthrough`, `Replace`, `Drop`, and `Block` by mutating the live Pingora body
chunk (`&mut Option<Bytes>`) in place.
There is no separate writeback step for bodies, so a `Replace` or a `Drop` takes effect immediately on the streaming
chunk.

## Key Files

| File                                                                      | Role                                          |
|---------------------------------------------------------------------------|-----------------------------------------------|
| `crates/snakeway-wit/wit/`                                                | WIT interface definition                      |
| `crates/snakeway-core/src/execution/device/wasm/engine.rs`                | `WasmEngine`, pooling allocator, epoch ticker |
| `crates/snakeway-core/src/execution/device/wasm/wasm_device.rs`           | `WasmDevice` struct and `Device` trait impl   |
| `crates/snakeway-core/src/execution/device/wasm/state.rs`                 | `HostState` (host interface impl)             |
| `crates/snakeway-core/src/execution/device/wasm/lifecycle.rs`             | Snapshot building, result/patch application   |
| `crates/snakeway-core/src/execution/device/wasm/bindings.rs`              | `wasmtime::component::bindgen!` invocation    |
| `crates/snakeway-core/src/execution/device/core/registry.rs`              | Device loading and engine lifecycle           |
| `crates/snakeway-conf/src/types/specification/device/wasm_device_spec.rs` | Config spec (parsing, validation)             |
| `crates/snakeway-conf/src/types/runtime/device/wasm_device_config.rs`     | Config runtime type (lowering)                |
