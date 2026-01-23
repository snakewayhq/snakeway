---
title: Devices Overview
---


At the core of Snakeway's design is the **Device**. A device is a modular, high-performance unit of logic that
intercepts and processes traffic as it flows through the proxy.

### The Pipeline Model

Snakeway executes devices in a strict, ordered pipeline. This ensures that traffic behavior is deterministic and easy to
reason about. Instead of a complex web of middlewares, Snakeway uses a linear chain of operators.

When a request enters the system, it passes through the device pipeline multiple times at well-defined lifecycle phases.

### Lifecycle Hooks

A device can hook into several stages of the request and response journey:

- **`on_request`**: Executed as soon as the request headers are received. This is the first opportunity to inspect or
  modify the incoming request.
- **`on_stream_request_body`**: Executed for every chunk of the streamed request body (if there is one).
- **`before_proxy`**: Executed immediately before the request is forwarded to an upstream service. This is the last
  chance to enrich the request with headers or perform final routing logic.
- **`after_proxy`**: Executed after receiving the response headers from the upstream, but before any processing occurs.
- **`on_response`**: Executed just before the response is sent back to the client. This is the final opportunity to
  modify headers or status codes.
- **`on_error`**: A specialized hook called if an error occurs during the pipeline execution, allowing devices to log or
  react to failures.

### Determinism and Order

The order of the devices in the pipeline is important.

The `Identity` device is always run first. It inspects the incoming request and extracts information about the client,
such as their IP address and user agent.

WASM devices are run after all other devices EXCEPT the `Structured Logging` device.
The order of WASM devices is determined by the order they are defined in your configuration.

The `Structured Logging` device is always run last.

This linear model eliminates the "" often found in complex middleware systems where execution order can be
unpredictable or dependent on an implicit internal state.

### Built-in vs. WASM

Snakeway provides two types of devices:

1. **Built-in Devices**: These are written in Rust and compiled directly into the Snakeway binary. They offer maximum
   performance for common tasks like identifying clients and logging.
2. **WebAssembly (WASM) Devices**: These are external modules that can be loaded at runtime. They allow you to extend
   Snakeway with custom logic written in various languages, running in a secure, sandboxed environment.

By combining these two types of devices, you can build a traffic pipeline that is both incredibly fast and infinitely
extensible.
