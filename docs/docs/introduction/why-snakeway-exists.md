---
slug: /
title: Why Snakeway Exists
---

The landscape of reverse proxies and API gateways is vast, ranging from simple, battle-tested tools like nginx to
massive, service-mesh architectures like Envoy and Istio. Snakeway exists to fill the gap between these two extremes.

## The Problem: Power vs. Complexity

When teams need to add custom logic to their edge (.e.g., request enrichment, custom access rules, or complex
observability), they often face a challenging choice:

1. **Simple Proxies**: Fast and reliable, but extending them often requires writing C modules or using limited scripting
   languages (like Lua), which can be challenging to test and maintain.
2. **Heavy Gateways**: Incredibly powerful, but often come with massive operational overhead, complex DSLs, and a "black
   box" nature that makes debugging difficult.

## The Snakeway Approach

Snakeway was built on a different set of priorities.

#### Programmability

Many problems can be solved more efficiently at the edge.
Programmability is the key that unlocks that door.

By using Rust and WASM, Snakeway allows developers to write, test, and deploy complex traffic logic using modern tools
and workflows.

#### Deterministic Pipeline

The order of operations should be explicit and easy to reason about.

Snakeway's linear device pipeline eliminates the "magic" of middleware.

#### Simplicity

Understanding other technologies is not a prerequisite to using Snakeway.

You do not need to know about Lua, Helm, Kubernetes, Terraform, or anything else.

#### Operator Experience

The directory-based configuration and modular design are built for humans, not just machines.

The [HCL configuration language](https://github.com/hashicorp/hcl) was carefully selected for its simplicity,
expressiveness, and ubiquity in the ops-space.

CLI commands allow for generating and inspecting configs.

#### Native Performance

Built on Pingora and Rust, Snakeway delivers the performance required for high-traffic environments without compromising
on safety or extensibility.

