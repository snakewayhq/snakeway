# Architecture

Snakeway is designed to be a high-performance, modular, and extensible traffic control engine. Its architecture
leverages the strengths of the Rust programming language and the Pingora proxy framework to deliver a robust and
scalable solution for modern edge computing.

### Built on Pingora

At its core, Snakeway is built on **Pingora**, Cloudflare's open-source Rust framework for building fast, reliable, and
customizable HTTP proxies. By leveraging Pingora, Snakeway inherits:

- **High-Performance HTTP Parsing**: Extremely fast and secure parsing of HTTP/1 and HTTP/2.
- **Asynchronous Runtime**: Built on Top of Tokio, allowing Snakeway to handle thousands of concurrent connections with
  minimal overhead.
- **Upstream Connection Pooling**: Efficient management of connections to backend services.
- **Modern TLS**: Native support for high-performance TLS via OpenSSL or BoringSSL.

### Snakeway vs. Snakeway-Core

The project is split into two primary components:

1. **`snakeway` (The Binary)**: This is the command-line interface and entry point. It handles configuration loading,
   logging initialization, and lifecycle management (starting, stopping, and reloading the server).
2. **`snakeway-core` (The Engine)**: This library contains the core logic of the proxy. It includes the router, the
   device pipeline, the configuration engine, and the management of upstream services.

This separation allows for better testability and enables the core engine to be used as a library in other Rust projects
if desired.

### The Request Flow

When a request enters Snakeway, it follows a deterministic path through the system:

1. **Listener**: The request is accepted by a network listener (HTTP or HTTPS).
2. **Router**: The router inspects the request path and determines which route and service should handle the request.
3. **Device Pipeline (Request Phase)**: The request passes through the `on_request` and `before_proxy` hooks of all
   enabled devices.
4. **Upstream Proxy**: If the route is a service route, the request is forwarded to an upstream service.
5. **Device Pipeline (Response Phase)**: The response from the upstream (or a static file handler) passes through the
   `after_proxy` and `on_response` hooks.
6. **Client Response**: The final response is sent back to the client.

### Component Map

- **Router**: Uses a fast radix-tree based matcher to map paths to handlers.
- **Traffic Manager**: Maintains a real-time snapshot of system health, upstream status, and performance metrics.
- **Device Registry**: Manages the lifecycle of both built-in and WASM devices.
- **Admin Gateway**: A specialized, terminal gateway that handles administrative requests and provides access to the
  system's internal state.
