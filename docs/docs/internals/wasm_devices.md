---
title: WASM Devices
---

:::note
This is a draft doc on how to evolve the experimental WASM feature.  
:::

Pre v1, WASM devices are an experimental prototype. They need to be evolved beyond this.

What needs to be done (in no exact order):

1. Per-request performance needs to be acceptable, right now it is not suitable for any production use case.
2. A complex WASM device needs to be authored to proof out the concept and catch design flaws/limitations.
   JWT auth is a good candidate.
3. Plugin lifecycle needs to be proofed out as well, especially with regard to hot reload.

Some notes on how to approach this (from the roadmap):

- Pre-instantiated components (no per-request instantiation)
- Bounded store pool with memory and execution limits
- Wasmtime caching and pooling allocator
- Per-hook timeouts and fail-open/fail-closed configuration
- Header and path mutation guardrails
- Plugin versioning and reload validation

