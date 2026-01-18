---
title: Why Snakeway Exists
---


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

### Why Rust and WASM?

**Rust** was chosen for the core of Snakeway because it offers the performance of C and C++ without the memory safety
risks. This allows for a proxy that is both incredibly fast and inherently secure.

**WebAssembly (WASM)** was chosen for extensibility because it provides a near-perfect sandbox.
Custom, third-party, or experimental logic can be run at the edge without any risk of crashing the core proxy or leaking
memory.
WASM also has the benefit of authorship in multiple languages (e.g., Rust, Go, Elixir, Python).
This makes a rich ecosystem of plugins and integrations possible.

### Who is Snakeway for?

Snakeway is for teams that:

- Need more than a simple proxy but don't want the weight of a full service mesh.
- Want to move traffic logic out of their application code and into the edge.
- Value transparency, observability, and deterministic behavior.
- Want to use modern languages like Rust or Go to control their traffic.

Snakeway isn't just a proxy. It's a **traffic control platform** designed to grow with your needs.
