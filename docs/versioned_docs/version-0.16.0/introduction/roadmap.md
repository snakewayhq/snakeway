---
title: Roadmap
---

This page outlines the development phases of Snakeway, from initial foundation through the 1.0 release and beyond.
Each phase has a defined set of goals and deliverables.
Completed items are marked with checkboxes.

## Milestone 0: Foundation (v0.1.0)

**Goals**

- [x] Create a functional binary (`snakeway`)
- [x] Implement a minimal config format (TOML)
- [x] Integrate Pingora with downstream and upstream HTTP/1.1 and HTTP/2
- [x] Implement basic reverse proxy routing for a single upstream
- [x] Implement basic device API
- [x] Structured logging via the tracing framework
- [x] CI and cross-compilation

**Deliverables**

- GitHub repository with a working MVP
- GitHub Actions CI pipeline
- Example configurations
- Linux release binaries

## Milestone 1: Foundations and extensibility (v0.2.x)

**Goals**

- [x] Define the Snakeway Device API (builtin Rust devices and WASM devices)
- [x] Add hot reload support (`snakeway reload` and SIGHUP)
- [x] Add static file serving with caching, compression, and range requests
- [x] Implement the Identity device

**Deliverables**

- Complete device API with lifecycle hooks
- WASM device loading via Wasmtime
- Static file server with ETag, If-Modified-Since, gzip, brotli, and range request support
- Hot reload via signal and CLI command

## Milestone 1.5: Benchmark and architecture review

Confirm that the architecture is sound before building on top of it.

- [x] Begin benchmark suite
- [x] Evaluate performance bottlenecks
- [x] Review error handling in the device lifecycle

## Milestone 2: Load balancing and observability (v0.3.x, v0.4.x)

**Goals**

- [x] Load balancing with multiple strategies (round-robin, least-connections, randomized)
- [x] Passive health checks and circuit breaking
- [x] Observability via admin API endpoints
- [x] Multiple upstream support with ordered failover
- [x] Downstream and upstream TLS
- [x] WebSocket and gRPC proxy support

**Deliverables**

- Upstream pool manager with health-check worker loop
- Circuit breaker with configurable thresholds
- Admin API: `/admin/health`, `/admin/upstreams`, `/admin/stats`, `/admin/reload`

## Milestone 2.5: Outstanding tasks (v0.5.x)

**Traffic Management**

- [x] Weighted load balancing strategy
- [x] Config-defined weights for A/B testing

**Configuration**

- [x] Audit validation coverage across all config sections
- [x] Cross-field and reload safety rules
- [x] Config observability improvements (`config dump` formatting)

**Architecture**

- [x] Review ownership, lifetimes, and error model
- [x] Define explicit device ordering mechanism
- [x] ALPN policy for downstream and upstream protocol negotiation
- [x] Refactor HttpProxy to focused components

**Routing**

- [x] Evaluate and document path matching precedence rules

## Milestone 3: Security and path control (v0.6.x)

**Goals**

- [x] Request normalization (path collapse, UTF-8 enforcement, query canonicalization)
- [x] CIDR-based network policies (allow/deny)
- [x] Method and header allowlists
- [x] Request size limits and rate limiting

**Deliverables**

- L4 connection rate limiting filter
- L4 network connection filter (CIDR)
- L7 request rate limiting device
- L7 network policy device
- Request filter device (methods, headers, body size)

## Milestone 3.1: Refinements (v0.7.x)

- [x] Standardize CLI format options between `config dump` and `config check`
- [x] Add `config init` command for first-proxy configuration generation
- [x] Rename `devices.d` to `device.d` for consistency
- [x] Separate remaining shared runtime/spec configuration state
- [x] Make `MAX_USER_AGENT_LENGTH` and `MAX_X_FORWARDED_FOR_LENGTH` configurable

## Milestone 3.2: Refinements (v0.8.x)

- [x] Add `work_stealing` toggle to server configuration

## Milestone 4: ACME TLS automation (v0.9.0)

**Goals**

- [x] Automated TLS certificate issuance via ACME (HTTP-01 challenge)
- [x] Automatic certificate renewal
- [x] Host-based route matching
- [x] `/admin/certs` endpoint for certificate inspection
- [x] `route solve` CLI command for debugging route matching
- [x] Review configuration lowering logic for safety

## Milestone 5: Hardening (v0.10.0)

All core features are implemented at this stage.
The focus shifts to architecture review, test coverage, and operational polish.

**Goals**

- [x] Clean, forward-looking architecture
- [x] Production-grade test suite (150+ integration tests)
- OpenTelemetry support
    - [x] OTLP export (traces, logs, metrics)
    - [x] W3C Trace Context propagation
    - [x] Configurable sampling (parent-based with trace-ID ratio)
    - [x] Per-phase child spans (routing, upstream selection, upstream request/response, response)
    - [x] Metrics instrumentation (request throughput, latency, errors, upstream health, circuit breaker)

**Configuration**

- [x] Consider moving validation logic into spec files where appropriate
- [x] Evaluate env var, CLI, and config parity.
  The current split is intentional (env vars for logging and ops, HCL for app behavior, CLI for paths)
- [x] Require pre-provisioned ACME cert_dir and data_dir (stop auto-creating directories), matching certbot behavior
- [x] Lazy DNS resolution for hosts (compatible with container environments)

**Devices**

- [x] Make UA Parser regex file overridable in the config (similar to MMDB files)
- [x] Review device subsystem against the mature configuration subsystem
- [x] Consider discrete `on_response_header` and `on_response_body`.
  Implemented `on_stream_response_body` instead.
- [x] Consider scoping network policy, request filter, and rate limiting devices to specific paths

**Routing**

- [x] Review routing code for conceptual duplication: reviewed, no changes needed.
  Structural parallelism between Static/Service routes is intentional
- [x] Implement more robust path matching

## Milestone 6: Packaging and distribution (v0.11.0)

**Goals**

- [x] `.deb` and `.rpm` packages
- [x] Systemd service unit
- [x] Distroless Docker images

**Deliverables**

Standard installation layout:

```
/etc/snakeway/snakeway.hcl
/etc/snakeway/ingress.d/*.hcl
/etc/snakeway/device.d/*.hcl
```

## Milestone 7: Reconsidered late additions (v0.12.0)

**Goals**

- [x] Zero-drop reload support for configuration changes under load.
- [x] Admin API authentication (bearer-token scheme, required on every `bind_admin`).
- [x] Make a config directory configurable with an environment variable and use it in packaging.
    - This solves an ergonomics issue where an operator has to specify the non-default values at the CLI per environment when troubleshooting a setup (which is annoying).

## Milestone 8: Alpha hardening and refinements (v0.13.0)

**Goals**

- [x] Move config validation primitives to discrete crate.
- [x] Rework config validation report collection to avoid a monolithic file that lists all possible issues.
- [x] Add sensible defaults and env vars after walking through real world deployment scenarios.
- [x] Allow HTTP/2-capable listener to proxy to a plaintext HTTP/1.1 origin

## Milestone 9: Rework conf subsystem (v0.14.0)

**Goals**

- [x] Rework the `confval` crate to replace the `Origin` mechanism with true per-line config validation issue provenance messages.
- [x] Create a `confval-derive` companion crate for confval that replaces o2o.

## Milestone 10: Full programmability (v0.15.0)

**Goals**

- [x] Expose some HTTP/2 fine-tuning options
- [x] Full WASM Device support

**What Full WASM Device Support looks like...**

This is bumped up ahead of the v1.0 release because it does not make sense to release a stable version of a programmable proxy if the programmability features are still experimental.

- Pre-instantiated components (no per-request instantiation)
- Bounded store pool with memory and execution limits
- Wasmtime caching and pooling allocator
- Per-hook timeouts and fail-open/fail-closed configuration
- Header and path mutation guardrails
- Plugin versioning and reload validation

## Milestone 11: Protocol negotiation (v0.16.0)

**Goals**

- [x] Replace the implicit HTTP version and upgrade negotiation logic in `HttpProxy` and `RequestCtx` with explicit state machines (`ProtocolMode` and `UpgradeState`).
  See [Protocol negotiation](../internals/protocol-negotiation.md) for the states, transitions, and rejection points.

## Milestone N: Snakeway 1.0

**Goals**

- [ ] Comprehensive documentation site
- [ ] Full operator manual
- [ ] Benchmark suite with published results
- [x] Stabilized device API

## Post-1.0

The following items are not in the critical path for 1.0 but represent the longer-term direction.

### Router performance

LRU cache in front of the router to make route lookups O(1) instead of O(n).
This should be profiled before and after implementing.
There may be no meaningful difference with a practical number of routes.

### Caching device

HTTP response caching using Pingora's native cache subsystem with pluggable storage (memory, disk, Redis, or custom backends).

Use Pingora Native HTTP Cache.

Rough draft of approach:

1. identity
2. rate_limit
3. OTHER_DEVICES
4. cache_lookup ← early device
5. origin call
6. cache_store ← response device
7. logging

Pluggable storage (supported by Pingora):

1. memory (LRU)
2. memory and disk
3. Redis
4. custom storage

### Active health checks

Background probe model (HTTP/TCP) independent of request traffic.
Passive health checks already exist.

### Additional certificate management

- PostgreSQL and/or S3 certificate stores
- DNS-01 ACME challenge support

### Kubernetes ingress controller

Optional feature that allows Snakeway to function as a Kubernetes ingress controller, polling for configuration changes and applying runtime snapshots through the existing configuration pipeline.

### Static file server enhancements

- Precompressed asset serving (`.br`, `.gz`)
- Zero-copy serving via `sendfile`
- WASM hooks for static file requests
- Per-file caching headers

### External control planes and discovery

- Dynamic certificate management
- Service discovery via DNS A/AAAA with TTL, SRV records, plugin-based discovery, and file-based watchers

### Admissions control

Standalone backpressure monitoring tool that integrates with the proxy for graceful load shedding.

### Packaging

- Helm chart
