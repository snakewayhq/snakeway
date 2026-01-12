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

Snakeway is:

- A **reverse proxy** built on a modern, high-performance runtime
- A **programmable request/response pipeline**
- A **host for user-defined logic** (built-in, WASM, or native)
- A **control plane at the edge**, not an application server

Snakeway is designed for teams that need **deterministic control over traffic behavior** without embedding that logic
deep inside every service.

## The Core Extensibility Model: Devices

Everything in Snakeway revolves around **devices**.

A device is a unit of logic that runs at a specific point in the request lifecycle.

Devices can:

- Read or modify request headers and bodies
- Make routing decisions
- Short-circuit requests with a response
- Observe traffic for logging or metrics
- React to upstream responses or errors

Devices are executed **in a strict, ordered pipeline**.

Order matters. Behavior is deterministic.

This is not middleware in the traditional web-framework sense.
Devices are closer to **traffic operators** than request handlers.

## Where Snakeway Fits

A typical deployment looks like this:

```
Web Browser -> Snakeway -> Upstream Web Services
```

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

Existing tools tend to fall into two camps:

1. **Simple proxies** that are fast but rigid
2. **Large gateway systems** that are powerful but heavy

Snakeway exists in the middle.

It prioritizes:

- A **clear mental model**
- **Extensibility without complexity**
- **Performance without magic**
- **Configuration that reflects intent**

Instead of shipping a fixed feature set, Snakeway ships a **stable execution model** and lets you build what you need on
top.

## Design Principles

Snakeway is built around a few non-negotiable ideas:

- **Explicit over implicit**  
  Nothing happens unless you configure it.

- **Programmable, not declarative-only**  
  Real logic requires real code.

- **Safe by default**  
  User-defined logic runs in constrained environments.

- **Observable from day one**  
  Traffic you can't see is traffic you don't control.

## How to Read the Docs

If you're new to Snakeway, read these pages next:

1. **[Mental Model](/guide/mental-model)** how requests flow through the system
2. **[Architecture](/guide/architecture)** how Snakeway is structured internally
3. **[Devices Overview](/devices/overview)** how extensibility works
4. **[Getting Started](/getting-started/installation)** running your first proxy
