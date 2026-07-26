---
slug: /
title: Why Snakeway Exists
---

The landscape of reverse proxies and API gateways is vast, ranging from simple, battle-tested tools like nginx to large service-mesh architectures like Envoy and Istio.
Snakeway exists to fill the gap between these two extremes.

## The Problem: Power vs. Complexity

When teams need to add custom logic to their edge (e.g., request enrichment, custom access rules, or complex observability), they often face a difficult choice:

1. **Simple proxies** are fast and reliable, but extending them often requires writing C modules or using limited
   scripting languages like Lua, which can be hard to test and maintain.
2. **Heavy gateways** are powerful, but often bring significant operational overhead, complex DSLs, and opaque behavior
   that makes debugging difficult.

## The Snakeway Approach

Snakeway was built on a different set of priorities.

### Programmability

You can solve many problems more efficiently at the edge.
Snakeway uses Rust and WASM so you can write, test, and deploy traffic logic with modern tools and workflows.

### Deterministic Pipeline

The order of operations is explicit and easy to reason about.
Snakeway runs devices in a single linear pipeline, so the sequence a request passes through is the sequence you wrote in the configuration.

### Simplicity

Understanding other technologies is not a prerequisite to using Snakeway.
You do not need to know Lua, Helm, Kubernetes, or Terraform.

### Operator Experience

The directory-based configuration and modular design are built for humans, not just machines.
The [HCL configuration language](https://github.com/hashicorp/hcl) was chosen for its simplicity, expressiveness, and ubiquity in operations.
CLI commands generate and inspect configuration.

### Native Performance

Built on Pingora and Rust, Snakeway delivers the performance required for high-traffic environments without compromising on safety or extensibility.

