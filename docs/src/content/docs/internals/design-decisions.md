---
title: Design Decisions
---


This document outlines the key architectural and technical decisions made during the development of Snakeway. These
decisions reflect our commitment to performance, safety, and developer experience.

### Atomic Hot Reloads with `arc-swap`

One of Snakeway's core features is the ability to reload configuration and devices without dropping connections. To
achieve this safely in a multi-threaded environment, we use the `arc-swap` crate.

- **The Problem**: Updating global state (like a router or device registry) while multiple worker threads are actively
  reading from it is a classic concurrency challenge.
- **The Solution**: We wrap the `RuntimeState` in an `ArcSwap`. When a reload is triggered, we build the *entire* new
  state offline. Once validated, we perform a single, atomic swap. Existing requests continue to use the "old" state
  until they complete, while all new requests immediately begin using the "new" state.
- **Result**: Zero-downtime reloads with minimal performance impact on the hot path.

### Directory-Based Configuration

While many proxies use a single, large configuration file, Snakeway favors a directory-based approach.

- **Modularity**: Large environments can have hundreds of routes and services. Splitting these into individual files (
  e.g., `routes.d/api.toml`, `routes.d/assets.toml`) makes them easier to manage, version control, and automate.
- **Discovery**: By using glob patterns in the `[include]` section, Snakeway can automatically discover new
  configuration files as they are added to the system, simplifying deployment pipelines.

### Choosing Pingora over Nginx or Envoy

Snakeway is built on Pingora, Cloudflare's Rust-based proxy framework.

- **Safety**: Building in Rust provides a level of memory safety that is difficult to achieve in C-based proxies like
  Nginx.
- **Modernity**: Pingora was designed from the ground up for modern cloud environments, with first-class support for
  HTTP/2 and asynchronous programming.
- **Customizability**: Unlike "black-box" proxies, Pingora provides a library-first approach, giving us the control
  needed to build the Device system and custom routing logic while still benefiting from a battle-tested HTTP engine.

### WASM for Extensibility

Instead of a custom DSL or embedded scripting language (like Lua or JavaScript), Snakeway uses WebAssembly (WASM).

- **Performance**: WASM offers near-native execution speed.
- **Isolation**: Each WASM device runs in its own sandbox, ensuring that a memory leak or crash in custom logic cannot
  affect the rest of the proxy.
- **Ecosystem**: WASM is a standard with growing industry support, allowing developers to use the languages and tools
  they already know.
