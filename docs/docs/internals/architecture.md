---
title: Architecture
---

Snakeway is a traffic control engine written in Rust and built on the Pingora proxy framework.
This page describes the crates it is split into, the path a request takes through the system, and the components that path passes through.

## Built on Pingora

At its core, Snakeway is built on **Pingora**, Cloudflare's open-source Rust framework for building HTTP proxies.
Pingora supplies the layers below the device pipeline:

- **HTTP parsing**: parsing of HTTP/1 and HTTP/2 request and response framing.
- **Asynchronous runtime**: Tokio, so a single process handles many concurrent connections without a thread per connection.
- **Upstream connection pooling**: reuse of established connections to backend services.
- **TLS**: termination and origination through OpenSSL or BoringSSL.

## Crate layout

The workspace splits into a binary crate and the libraries it drives.

- **`snakeway`** is the binary.
  It provides the command-line interface, loads configuration, initializes logging, and runs the control plane that starts, stops, and reloads the server.
- **`snakeway-engine`** holds the request-processing core.
  It contains the router, the device pipeline and registry, the request and response contexts, and the traffic manager that tracks upstream health and selection.
- **`snakeway-conf`** parses HCL, validates it, and lowers the operator-facing spec types into the runtime config types the rest of the workspace reads.
- **`snakeway-proxy`** implements the Pingora `ProxyHttp` service, the static file handler, the admin handler, and the bootstrap and zero-drop upgrade paths.
- **`snakeway-net`** holds the connection-level primitives: CIDR matching, client IP resolution, the network connection filter, and the connection rate limiting filter.
- **`snakeway-acme`** runs ACME certificate issuance, the certificate and order stores, and the renewal scheduler.
- **`snakeway-observability`** initializes logging, metrics, and OpenTelemetry export, and propagates W3C trace context across the proxy hop.

The engine does not depend on the binary, so you can drive the pipeline from a test harness or another Rust program without starting the CLI.

## The request flow

When a request enters Snakeway, it follows a deterministic path through the system:

1. **Listener**: the request is accepted by a network listener (HTTP or HTTPS).
2. **Router**: the router inspects the request path and determines which route and service should handle the request.
3. **Device pipeline (request phase)**: the request passes through the `on_request`, `on_stream_request_body`, and `before_proxy` hooks of all enabled devices.
4. **Upstream proxy**: if the route is a service route, the request is forwarded to an upstream service.
5. **Device pipeline (response phase)**: the response from the upstream or from the static file handler passes through the `after_proxy` and `on_response` hooks.
6. **Client response**: the final response is sent back to the client.

## Component map

- **Router**: uses longest-path matching to route requests to the appropriate service or static file handler.
- **Traffic manager**: maintains the current snapshot of system health, upstream status, and performance counters.
- **Device registry**: manages the lifecycle of both builtin and WASM devices.
- **Admin proxy**: a terminal proxy that handles administrative requests and exposes the system's internal state.
