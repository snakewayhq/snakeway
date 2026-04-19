---
title: Roadmap
---

This page outlines the development phases of Snakeway, from initial foundation through the 1.0 release and beyond. Each
phase has a defined set of goals and deliverables. Completed items are marked with checkboxes.

---

## Phase 0: Foundation (v0.1.0)

**Goals**

- [x] Create a functional binary (`snakeway`)
- [x] Implement a minimal config format (TOML)
- [x] Integrate Pingora with downstream and upstream HTTP/1.1 + HTTP/2
- [x] Implement basic reverse proxy routing for a single upstream
- [x] Implement basic device API
- [x] Structured logging via the tracing framework
- [x] CI and cross-compilation

**Deliverables**

- GitHub repository with a working MVP
- GitHub Actions CI pipeline
- Example configurations
- Linux release binaries

---

## Phase 1: Foundations and Extensibility (v0.2.x)

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

---

## Phase 1.5: Benchmark and Architecture Review

Confirm that the architecture is sound before building on top of it.

- [x] Begin benchmark suite
- [x] Evaluate performance bottlenecks
- [x] Review error handling in the device lifecycle

---

## Phase 2: Load Balancing and Observability (v0.3.x, v0.4.x)

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

---

## Phase 2.5: Outstanding Tasks (v0.5.x)

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

---

## Phase 3: Security and Path Control (v0.6.x)

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

---

## Phase 3.1: Refinements (v0.7.x)

- [x] Standardize CLI format options between `config dump` and `config check`
- [x] Add `config init` command for first-proxy configuration generation
- [x] Rename `devices.d` to `device.d` for consistency
- [x] Separate remaining shared runtime/spec configuration state
- [x] Make `MAX_USER_AGENT_LENGTH` and `MAX_X_FORWARDED_FOR_LENGTH` configurable

---

## Phase 3.2: Refinements (v0.8.x)

- [x] Add `work_stealing` toggle to server configuration

---

## Phase 4: ACME TLS Automation (v0.9.0)

**Goals**

- [x] Automated TLS certificate issuance via ACME (HTTP-01 challenge)
- [x] Automatic certificate renewal
- [x] Host-based route matching
- [x] `/admin/certs` endpoint for certificate inspection
- [x] `route solve` CLI command for debugging route matching
- [x] Review configuration lowering logic for safety

---

## Phase 5: Hardening (v0.10.0)

All core features are implemented at this stage. The focus shifts to architecture review, test coverage, and operational
polish.

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
- [x] Evaluate env var / CLI / config parity - current split is intentional (env vars for logging/ops, HCL for app
  behavior, CLI for paths)
- [x] Require pre-provisioned ACME cert_dir and data_dir (stop auto-creating directories) - matches certbot behavior
- [x] Lazy DNS resolution for hosts (compatible with container environments)

**Devices**

- [x] Make UA Parser regex file overridable in the config (similar to MMDB files)
- [x] Review device subsystem against the mature configuration subsystem
- [x] Consider discrete `on_response_header` and `on_response_body` - implemented `on_stream_response_body` instead.
- [x] Consider scoping network policy, request filter, and rate limiting devices to specific paths

**Routing**

- [x] Review routing code for conceptual duplication: reviewed, no changes needed; structural parallelism between
  Static/Service routes is intentional
- [x] Implement more robust path matching

---

## Phase 6: Packaging and Distribution

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

---

## Phase 7: Snakeway 1.0

**Goals**

- [ ] Comprehensive documentation site
- [ ] Full operator manual
- [ ] Benchmark suite with published results
- [x] Stabilized device API

---

## Post-1.0

The following items are not in the critical path for 1.0 but represent the longer-term direction.

### Enhanced Hot Reload

Zero-drop reload support for seamless configuration changes under load.

### Router Performance

LRU cache in front of the router to make route lookups O(1) instead of O(n).

### Caching Device

HTTP response caching using Pingora's native cache subsystem with pluggable storage (memory, disk, Redis, or custom
backends).

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
2. memory + disk
3. Redis
4. custom storage

### Full WASM Device Support

- Pre-instantiated components (no per-request instantiation)
- Bounded store pool with memory and execution limits
- Wasmtime caching and pooling allocator
- Per-hook timeouts and fail-open/fail-closed configuration
- Header and path mutation guardrails
- Plugin versioning and reload validation

### Active Health Checks

Background probe model (HTTP/TCP) independent of request traffic. Passive health checks already exist.

### Additional Certificate Management

- PostgreSQL and/or S3 certificate stores
- DNS-01 ACME challenge support

### Kubernetes Ingress Controller

Optional feature that allows Snakeway to function as a Kubernetes ingress controller, polling for configuration changes
and applying runtime snapshots through the existing configuration pipeline.

### Static File Server Enhancements

- Precompressed asset serving (`.br`, `.gz`)
- Zero-copy serving via `sendfile`
- WASM hooks for static file requests
- Per-file caching headers

### External Control Planes and Discovery

- Dynamic certificate management
- Service discovery via DNS A/AAAA with TTL, SRV records, plugin-based discovery, and file-based watchers

### Admissions Control

Standalone backpressure monitoring tool that integrates with the proxy for graceful load shedding.

### Packaging

- Helm chart
