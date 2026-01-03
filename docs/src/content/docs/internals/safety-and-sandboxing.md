---
title: Safety and Sandboxing
---


Security and stability are core tenets of Snakeway. When building a programmable proxy, it is essential to ensure that
user-defined logic cannot compromise the integrity of the system or other requests.

### WASM Isolation with Wasmtime

Snakeway uses **Wasmtime**, a high-performance WebAssembly runtime, to execute custom device logic. Wasmtime provides a
robust sandbox that ensures:

- **Memory Safety**: WASM modules have their own isolated linear memory. They cannot access the proxy's internal memory
  or the memory of other requests.
- **Resource Limits**: We can enforce limits on CPU and memory usage for each WASM execution, preventing "noisy
  neighbor" scenarios or infinite loops from affecting the server's stability.
- **Capability-Based Security**: WASM modules can only interact with the host (Snakeway) through the
  well-defined [WIT interface](/devices/wasm). They have no direct access to the filesystem, network, or system calls
  unless explicitly granted.

### Pipeline Error Handling

Stability is further enhanced by how Snakeway handles errors within the device pipeline.

- **Non-Fatal Errors**: If a device hook encounters an error (e.g., a WASM module panics), Snakeway catches the error
  and calls the device's `on_error` hook. By default, the rest of the pipeline continues to execute, ensuring that a
  single failing device doesn't block the entire request journey.
- **Fail-Safe Defaults**: We prioritize service availability. If a device responsible for enrichment fails, the request
  still reaches the upstream, albeit without the enrichment. This "fail-open" or "fail-closed" behavior is configurable
  depending on the device's intent.

### Rust's Memory Safety

The core of Snakeway is written in Rust, providing native protection against common security vulnerabilities:

- **No Buffer Overflows**: Rust's ownership and borrowing system eliminates the risk of memory corruption.
- **Thread Safety**: The compiler ensures that data shared across worker threads is done so safely, preventing race
  conditions in the request pipeline.
- **Zero-Cost Abstractions**: We get these safety benefits without sacrificing the raw performance required for an edge
  proxy.

### Future: Request Isolation

We are exploring further isolation techniques, such as running individual request pipelines in separate tasks with
dedicated resource budgets, to provide even stronger guarantees in highly multi-tenant environments.
