---
layout: home

title: Snakeway
titleTemplate: false

hero:
  name: Snakeway
  text: Programmable Edge Proxy
  tagline: A modern, extensible reverse proxy built on Pingora
  image:
    src: /logo.svg
    alt: Snakeway logo
  actions:
    - theme: brand
      text: Get Started
      link: /overview
    - theme: alt
      text: GitHub
      link: https://github.com/ethanhann/snakeway

features:
  - icon: 🧩
    title: Device-Based Architecture
    details: Intercept, inspect, and modify requests at well-defined lifecycle phases using built-in Rust or WASM devices.

  - icon: 🔄
    title: Hot Reload, Zero Downtime
    details: Reload configuration and devices via signal or admin API without restarting the server.

  - icon: 📁
    title: High-Performance Static Files
    details: Native file serving with ETag, compression, range requests, and cache control. No upstream required.

  - icon: 📊
    title: Observability First
    details: Structured logs and admin endpoints designed for real production debugging.

  - icon: 🦀
    title: Built on Pingora
    details: Leverages Cloudflare’s Pingora 0.6.0 for serious throughput and modern HTTP support.

  - icon: 🛠️
    title: Designed to Evolve
    details: A forward-looking architecture that grows from edge proxy to full traffic intelligence platform.

footer:
  message: Released under the Apache 2.0 License
---

# Snakeway Overview

Snakeway is a programmable edge proxy built on top of **Pingora**.

Its goal is to provide **predictable extensibility** without sacrificing performance or correctness.

This document explains *how Snakeway thinks*.

## Design Goals

Snakeway is built around a few non-negotiable principles:

### 1. Explicit Execution

Every request follows a clearly defined lifecycle.  
There are no hidden hooks, background mutations, or implicit side effects.

```
on_request → before_proxy → after_proxy → on_response
```

If logic runs, it runs in one of these phases.

### 2. Devices, Not Scripts

Snakeway does not embed a general-purpose scripting language.

Instead, it uses **Devices**:

- Small, focused processing units
- Composable
- Ordered
- Explicitly scoped

Devices can:

- Inspect or modify requests
- Shape responses
- Block traffic
- Emit metrics
- Enrich request context (identity, geo, fraud signals)

Built-in devices are written in Rust.  
Third-party logic can be loaded via WASM (optional, sandboxed).

### 3. Request-Scoped Context

Each request carries a mutable, request-scoped context.

- No global state
- No thread-local hacks
- No implicit mutation

Devices communicate by extending this context in a controlled way.

## Hot Reload Model

Snakeway supports hot reload via `SIGHUP`:

```shell
snakeway reload
```

On reload:

1. Configuration is parsed and validated
2. Routes are rebuilt
3. Device registry is reconstructed
4. New state is atomically swapped in

Existing requests complete normally.  
New requests see the updated configuration.

## Observability First

Snakeway treats observability as a first-class concern.

Out of the box:

- Structured logs
- Request counters
- Upstream latency tracking
- Error metrics

Metrics are designed to be consumed by external systems rather than hidden behind dashboards.

## What Snakeway Is Not

Snakeway intentionally avoids:

- Implicit global scripting
- Runtime code mutation
- Control-plane-heavy architectures
- "Magic" auto-configuration

It favors correctness, debuggability, and long-term maintainability over convenience.

## Who Snakeway Is For

Snakeway is built for:

- Infrastructure engineers
- Edge and ad-tech workloads
- Teams who want control without chaos
- Developers who value explicit systems

If nginx is a wrench, Snakeway is a machine shop.
