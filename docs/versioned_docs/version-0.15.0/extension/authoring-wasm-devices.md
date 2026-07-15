---
title: Authoring WASM Devices
---

:::caution
The WIT interface (`snakeway:device@0.4.0`) may change in future releases.
:::

**WASM devices** let you extend Snakeway with custom traffic logic written in any language
that compiles to WebAssembly. Devices run in a sandboxed Wasmtime runtime with access to
request and response context, a host API for logging and metrics, and a typed configuration
map.

## Why WebAssembly

- **Language flexibility.** Write devices in Rust, Go, Zig, or any language that targets
  WASM components.
- **Isolation.** Each device runs in a memory-limited sandbox. A bug in a WASM device cannot
  crash the proxy.
- **Portable artifacts.** A compiled `.wasm` module runs on any platform Snakeway supports.

## Configuration

WASM devices are declared in a device file under `device.d/`:

```hcl
wasm_devices = [
  {
    name            = "my-device"
    enable          = true
    path            = "/etc/snakeway/devices/my_device.wasm"
    fail_policy     = "open"
    timeout_ms      = 5
    body_buffer_max = 0
    hooks = ["on_request"]

    config = {
      api_key = "secret-value"
      mode    = "strict"
    }
  }
]
```

### Fields

| Field             | Type                   | Required | Default | Description                                                                                                                    |
|-------------------|------------------------|----------|---------|--------------------------------------------------------------------------------------------------------------------------------|
| `name`            | string                 | yes      |         | Unique identifier for this device instance                                                                                     |
| `enable`          | bool                   | yes      |         | Whether the device is active                                                                                                   |
| `path`            | string                 | yes      |         | Path to the compiled `.wasm` component                                                                                         |
| `fail_policy`     | `"open"` or `"closed"` | yes      |         | Behavior on device error (see below)                                                                                           |
| `timeout_ms`      | integer                | no       | `5`     | Per-hook execution deadline in milliseconds (1 -- 60000)                                                                       |
| `body_buffer_max` | integer                | no       | `0`     | Max request body bytes to buffer before calling the body hook. `0` = streaming mode                                            |
| `config`          | map of strings         | no       | `{}`    | Key-value pairs accessible to the device via `host.config-get`                                                                 |
| `hooks`           | list of strings        | no       | all     | Lifecycle hooks this device implements. When set, the host skips every hook not listed. See [Hook Selection](#hook-selection). |

### Fail Policy

When a WASM device encounters an error (load failure, timeout, trap, body buffer overflow):

- **`"open"`** -- log a warning and continue processing. The device is skipped.
- **`"closed"`** -- log an error and return `503 Service Unavailable`. The request is blocked.

### Body Buffering

When `body_buffer_max` is `0` (the default), the `on-stream-request-body` hook is called
once per chunk as the body streams through. The device sees one chunk at a time.

When `body_buffer_max` is set to a positive value, Snakeway buffers the request body up to
that limit and calls `on-stream-request-body` once with the complete body when the stream
ends. If the body exceeds the limit, the fail policy determines the outcome.

### Hook Selection

By default, the host calls all six lifecycle hooks on every request.
Each call creates a fresh WASM instance, so a device that only implements one hook still pays for five no-op
instantiations per request.

The optional `hooks` field declares which hooks the device actually implements.
The host skips any hook not listed, creating no instance for it, which removes that overhead:

```hcl
hooks = ["on_request"]
```

Valid values are `on_request`, `on_stream_request_body`, `before_proxy`, `after_proxy`,
`on_stream_response_body`, and `on_response`.

Omitting `hooks` runs all six (the previous behavior).
An empty list is rejected; to disable a device set `enable = false`.

## The WIT Interface

The device interface is defined in `snakeway:device@0.4.0`. A WASM device imports a **host**
interface and exports a **policy** interface.

### World

```wit
world device {
  import host;
  export policy;
}
```

### Policy (exported by the device)

The device must export all six lifecycle hooks. Hooks that have no work to do should return
`continue` with no patch, and may be omitted from the device's `hooks` config so the host
skips calling them entirely (see [Hook Selection](#hook-selection)).

```wit
interface policy {
  on-request: func(req: request) -> request-result;
  on-stream-request-body: func(req: request, chunk: option<body-chunk>) -> body-result;
  before-proxy: func(req: request) -> request-result;
  after-proxy: func(resp: response) -> response-result;
  on-stream-response-body: func(resp: response, chunk: option<body-chunk>) -> body-result;
  on-response: func(resp: response) -> response-result;
}
```

Each hook corresponds to a phase in the [request lifecycle](/docs/internals/lifecycle).

### Host (imported by the device)

The host interface provides four functions the device can call during hook execution:

```wit
interface host {
  config-get: func(key: string) -> option<string>;
  log: func(level: u8, message: string);
  metric-increment: func(name: string, delta: u64);
  epoch-secs: func() -> u64;
}
```

| Function           | Description                                                                                            |
|--------------------|--------------------------------------------------------------------------------------------------------|
| `config-get`       | Read a value from the device's `config` map. Returns `none` if the key is absent.                      |
| `log`              | Emit a log message. Levels: `0` = trace, `1` = debug, `2` = info, `3` = warn, `4` = error.             |
| `metric-increment` | Increment a named counter. Exposed as `snakeway.wasm.device.custom` with `device` and `metric` labels. |
| `epoch-secs`       | Current Unix timestamp in seconds. Use for time-dependent logic (token expiry, rate windows).          |

### Types

#### Request and Response Snapshots

```wit
record request {
method: string,
scheme: string,
authority: string,
original-path: string,
route-path: string,
query: string,
client-ip: string,
headers: list<header>,
}

record response {
status: u16,
headers: list<header>,
}
```

Request snapshots are read-only views of the current request state. To mutate the request,
return a patch in the result.

#### Actions

Every request-phase hook returns a `request-result` and every response-phase hook returns a
`response-result`. Both contain an **action** and an optional **patch**.

```wit
variant action {
continue,
block,
respond(synthetic-response),
}
```

- **`continue`** -- proceed to the next device or phase. Apply the patch if present.
- **`block`** -- return `403 Forbidden` immediately.
- **`respond`** -- return a custom HTTP response with the given status, headers, and body.

#### Patches

Request patches can rewrite the route path, override the upstream path, and manipulate
headers:

```wit
record request-patch {
set-route-path: option<string>,
set-upstream-path: option<string>,
ops: list<header-op>,
}

variant header-op {
set(
header),
append(
header
),
remove(
string
),
}
```

Response patches can override the status code and manipulate response headers:

```wit
record response-patch {
set-status: option<u16>,
ops: list<header-op>,
}
```

#### Body Actions

Body hooks return a `body-result` with a `body-action`:

```wit
variant body-action {
passthrough,
replace(list<u8>
),
drop,
block,
}
```

- **`passthrough`** -- forward the chunk unmodified
- **`replace`** -- replace the chunk with the given bytes
- **`drop`** -- discard the chunk
- **`block`** -- return `403 Forbidden` immediately

## Building a Rust WASM Device

### Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- `cargo-component`: `cargo install cargo-component`
- The Snakeway WIT files from `crates/snakeway-wit/wit/`

### 1. Create the Project

```shell
mkdir my-device && cd my-device
cargo init --lib
```

Set the crate type in `Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
wit-bindgen = "0.57"
```

### 2. Generate Bindings and Implement the Device

In `src/lib.rs`, use `wit_bindgen::generate!` to create the bindings and implement the
`Guest` trait:

```rust
use wit_bindgen::generate;

generate!({
    path: "/path/to/snakeway-wit/wit/",
    world: "device",
});

use exports::snakeway::device::policy::Guest;
use snakeway::device::host;
use snakeway::device::types::{
    Action, BodyAction, BodyChunk, BodyResult, Header, HeaderOp,
    Request, RequestPatch, RequestResult, Response, ResponseResult,
};

struct MyDevice;

impl Guest for MyDevice {
    fn on_request(req: Request) -> RequestResult {
        host::log(2, &format!("request: {} {}", req.method, req.route_path));

        RequestResult {
            action: Action::Continue,
            patch: Some(RequestPatch {
                set_route_path: None,
                set_upstream_path: None,
                ops: vec![HeaderOp::Set(Header {
                    name: "x-wasm-device".to_string(),
                    value: "active".to_string(),
                })],
            }),
        }
    }

    fn on_stream_request_body(_req: Request, _chunk: Option<BodyChunk>) -> BodyResult {
        BodyResult { action: BodyAction::Passthrough }
    }

    fn before_proxy(_req: Request) -> RequestResult {
        RequestResult { action: Action::Continue, patch: None }
    }

    fn after_proxy(_resp: Response) -> ResponseResult {
        ResponseResult { action: Action::Continue, patch: None }
    }

    fn on_stream_response_body(_resp: Response, _chunk: Option<BodyChunk>) -> BodyResult {
        BodyResult { action: BodyAction::Passthrough }
    }

    fn on_response(_resp: Response) -> ResponseResult {
        ResponseResult { action: Action::Continue, patch: None }
    }
}

export!(MyDevice);
```

### 3. Build

```shell
cargo component build --release --target wasm32-unknown-unknown
```

The compiled component is at `target/wasm32-unknown-unknown/release/my_device.wasm`.

### 4. Deploy

Copy the `.wasm` file to a path accessible by Snakeway and add it to your device
configuration:

```hcl
wasm_devices = [
  {
    name        = "my-device"
    enable      = true
    path        = "/etc/snakeway/devices/my_device.wasm"
    fail_policy = "open"
  }
]
```

## Runtime Behavior

### Execution Model

Each hook invocation creates a fresh, isolated WASM instance. Instances do not share memory
across hooks or across requests. A **pooling allocator** pre-allocates memory slots so
instantiation is fast.

### Timeouts

Each hook has an epoch-based deadline controlled by `timeout_ms`. If the device exceeds its
deadline, the hook is terminated and the fail policy determines the outcome.

### Memory Limits

Each instance is limited to 10 MB of linear memory and 10,000 table elements. Exceeding
these limits triggers a trap, handled by the fail policy.

### Observability

Snakeway emits the following metrics for WASM devices:

| Metric                                  | Type      | Description                                              |
|-----------------------------------------|-----------|----------------------------------------------------------|
| `snakeway.wasm.device.hook_duration_ms` | histogram | Execution time per hook, labeled by device and hook name |
| `snakeway.wasm.device.failures`         | counter   | Failures labeled by device, hook, and reason             |
| `snakeway.wasm.device.custom`           | counter   | Guest-emitted metrics via `host.metric-increment`        |

Device log output (via `host.log`) appears in the Snakeway structured log stream with a
`device` field identifying the source.
