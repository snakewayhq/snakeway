---
title: WebAssembly Devices
---


Snakeway can be extended using WebAssembly (WASM), allowing you to write custom traffic logic in a variety of languages
while maintaining high performance and strong security isolation.

### Why WebAssembly?

By using WASM for extensibility, Snakeway provides several key benefits:

- **Language Flexibility**: You can write your devices in Rust, Go, Zig, or any other language that can compile to WASM.
- **Security and Isolation**: Custom logic runs in a secure, sandboxed environment. A bug in a WASM device cannot crash
  the entire proxy.
- **Near-Native Performance**: Modern WASM runtimes offer execution speeds that are very close to native code.
- **Portable Artifacts**: Compiled `.wasm` modules can be shared and deployed across different architectures without
  recompilation.

### The Snakeway WIT Definition

Snakeway defines the interface between the proxy and your WASM device using **WebAssembly Interface Type (WIT)**. This
ensures that your plugin and Snakeway agree on the data structures and functions used for communication.

The core of the interface is the `policy` interface, which includes hooks for the request and response phases:

```wit
interface policy {
  on-request: func(req: request) -> request-result;
  on-stream-request-body: func(req: request, chunk: body-chunk) -> body-result;
  before-proxy: func(req: request) -> request-result;
  after-proxy: func(resp: response) -> response-result;
  on-response: func(resp: response) -> response-result;
}
```

### Developing a Rust-Based WASM Device

To build a WASM device in Rust, you'll need the `cargo-component` tool and the Snakeway WIT files.

#### 1. Initialize a New Project

```bash
cargo component new my-wasm-device --lib
```

#### 2. Define the Logic

In your `src/lib.rs`, you can implement the Snakeway interface. Here is a simple example that adds a custom header to
every request:

```rust
use bindings::snakeway::device::policy::{Guest, Request, RequestResult, Decision, RequestPatch, Header};

struct MyDevice;

impl Guest for MyDevice {
    fn on_request(req: Request) -> RequestResult {
        RequestResult {
            decision: Decision::Continue,
            patch: Some(RequestPatch {
                set_headers: vec![Header {
                    name: "X-My-WASM-Header".to_string(),
                    value: "Hello from WASM!".to_string(),
                }],
                ..Default::default()
            }),
        }
    }

    // ... implement other hooks ...
}

bindings::export!(MyDevice with_types_in bindings);
```

#### 3. Compile to WASM

```bash
cargo component build --release
```

This will produce a `.wasm` file in `target/wasm32-wasi/release/`.

### Loading Your WASM Device

Once you have your compiled `.wasm` module, you can load it into Snakeway via your configuration:

```hcl
wasm_devices = [
  {
    enable = true
    path   = "/path/to/my_wasm_device.wasm"
    config = {
      key = "value"
    }
  }
]
```

For more details on the WIT definition and advanced WASM features, refer to the `snakeway-wit` directory in the Snakeway
repository.
