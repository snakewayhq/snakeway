---
title: Internals
---

## The Core Extensibility Model

Everything in Snakeway revolves around `devices`.

A device is a unit of logic that runs at a specific point in the request lifecycle.

Devices can:

Read or modify request headers and bodies
Make routing decisions
Short-circuit requests with a response
Observe traffic for logging or metrics
React to upstream responses or errors
Devices are executed in a strict, ordered pipeline.

Order matters. Behavior is deterministic.

This is not middleware in the traditional web-framework sense. Devices are closer to traffic operators than request
handlers.

## Why Rust and WASM?

**Rust** was chosen for the core of Snakeway because it offers the performance of C and C++ without the memory safety
risks. This allows for a proxy that is both incredibly fast and inherently secure.

**WebAssembly (WASM)** was chosen for extensibility because it provides a near-perfect sandbox.
Custom, third-party, or experimental logic can be run at the edge without any risk of crashing the core proxy or leaking
memory.

WASM also has the benefit of authorship in multiple languages (e.g., Rust, Go, Elixir, Python).
This makes a rich ecosystem of plugins and integrations possible.
