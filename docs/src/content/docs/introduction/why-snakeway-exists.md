---
title: Why Snakeway Exists
---

The landscape of reverse proxies and API gateways is vast, ranging from simple, battle-tested tools like Nginx to
massive, service-mesh architectures like Envoy and Istio. Snakeway exists to fill the gap between these two extremes.

## The Problem: Power vs. Complexity

When teams need to add custom logic to their edge (.e.g., request enrichment, custom access rules, or complex
observability), they often face a challenging choice:

1. **Simple Proxies**: Fast and reliable, but extending them often requires writing C modules or using limited scripting
   languages (like Lua), which can be challenging to test and maintain.
2. **Heavy Gateways**: Incredibly powerful, but often come with massive operational overhead, complex DSLs, and a "black
   box" nature that makes debugging difficult.

## The Snakeway Philosophy

Snakeway was built on a different set of priorities:

- **Programmability First**: Real logic requires a real programming language. By using Rust and WASM, Snakeway allows
  developers to write, test, and deploy complex traffic logic using modern tools and workflows.
- **Deterministic Pipeline**: The order of operations should be explicit and easy to reason about.
  Snakeway's linear device pipeline eliminates the "magic" of middleware.
- **Developer Experience**: Configuration should reflect intent. Our directory-based configuration and modular design
  are built for humans, not just machines.
- **Native Performance**: Built on Pingora and Rust, Snakeway delivers the performance required for high-traffic
  environments without compromising on safety or extensibility.

