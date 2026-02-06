---
title: What Is Snakeway?
---


Snakeway is a **programmable traffic control engine** that sits in front of your services and executes a configurable
pipeline of logic on every request and response.

At its core, Snakeway answers a simple question:

> *“What should happen to this request before, during, and after it reaches an upstream service?”*

Snakeway lets you answer that question with **devices** - small, composable units of logic that can observe, mutate,
short-circuit, or enrich traffic as it flows through the system.

## What Snakeway Is

- A **reverse proxy** built on a modern, high-performance runtime
- A **programmable request/response pipeline**
- A **host for user-defined logic** (built-in, WASM, or native)
- A **control plane at the edge**, not an application server

Snakeway is designed for teams that need **deterministic control over traffic behavior** without embedding that logic
deep inside every service.

## Where Snakeway Fits Architecturally

Snakeway sits **between clients and web services**, making decisions at the edge before traffic ever hits application
code.

Common use cases include:

- Structured access logging and observability
- Header normalization and enrichment
- Static file serving alongside proxied traffic
- Feature flags and traffic gating
- Rules engines and request classification
- Early rejection of invalid or abusive requests

Snakeway keeps this logic **out of your apps** and **out of your infrastructure glue code**.

## Why Snakeway Exists

The landscape of reverse proxies and API gateways is vast, ranging from simple, battle-tested tools like Nginx to
massive, service-mesh architectures like Envoy and Istio. Snakeway exists to fill the gap between these two extremes.

### The Problem: Power vs. Complexity

When teams need to add custom logic to their edge (.e.g., request enrichment, custom access rules, or complex
observability), they often face a challenging choice:

1. **Simple Proxies**: Fast and reliable, but extending them often requires writing C modules or using limited scripting
   languages (like Lua), which can be challenging to test and maintain.
2. **Heavy Gateways**: Incredibly powerful, but often come with massive operational overhead, complex DSLs, and a "black
   box" nature that makes debugging difficult.

### The Snakeway Philosophy

Snakeway was built on a different set of priorities:

- **Programmability First**: Real logic requires a real programming language. By using Rust and WASM, Snakeway allows
  developers to write, test, and deploy complex traffic logic using modern tools and workflows.
- **Deterministic Pipeline**: The order of operations should be explicit and easy to reason about.
  Snakeway's linear device pipeline eliminates the "magic" of middleware.
- **Developer Experience**: Configuration should reflect intent. Our directory-based configuration and modular design
  are built for humans, not just machines.
- **Native Performance**: Built on Pingora and Rust, Snakeway delivers the performance required for high-traffic
  environments without compromising on safety or extensibility.

## How to Read the Docs

If you're new to Snakeway, read these pages next:

1. **[Mental Model](/guide/mental-model)** how requests flow through the system
2. **[Architecture](/internals/architecture)** how Snakeway is structured internally
3. **[Devices Overview](/devices/overview)** how extensibility works
4. **[Getting Started](/getting-started/installation)** running your first proxy
