# Snakeway Roadmap

> A modern, extensible reverse proxy built on Pingora 0.6.0

## Phase 0: Foundation (v0.1.0)

### Goals

1. Create a functional binary: `snakeway`
2. Implement a minimal config format (TOML)
3. Integrate Pingora 0.6.0 with:
    * downstream HTTP/1.1 + HTTP/2
    * upstream HTTP/1.1 + HTTP/2
4. Implement basic reverse proxy routing for 1 upstream
5. Implement basic plugin/device API
6. Structured logging (tracing + JSON)
7. CI + cross-compilation set up

### **Deliverables**

* GitHub repo with a working MVP
* GitHub Actions CI
* Example configs
* Release assets (Linux binaries)

## Phase 1: Foundations and Extensibility (v0.2.x)

### Goals

* Fully define the **Snakeway Device API** (plugins)
* Add **hot reload** support
* Add **file server** support for static content
* Implement *real* observability stubs

### Features

Plugin/device phases:

* Device loading:
    * built-in Rust devices
    * dynamic WASM devices (WasmTime)
    * dynamic Rust plugins (optional)

* Hot reload with signal or admin API:
    * `snakeway reload`

* Observability:
    * structured logs

* Static file server:
    * basic file serving that preempts upstream
    * features:
        * ETag / If-Modified-Since
        * gzip / brotli
        * range requests
        * directory listing
        * per-file caching headers

### Deliverables

* Complete plugin API draft
* Example devices (header rewrite, logging)

### Implementation Order

#### Phase 1A \- Basic Lifecycle Plumbing and Config

1. Device API \+ ctx structures
2. Device registry \+ execution pipeline
3. Pipeline integration with Pingora
4. Built-in and WASM device API
5. Config loader (TOML)

#### Phase 1B \- Configuration and Static File Server

1. Static file server (basics)
2. Static file server (Etag, If-Modified-Since, gzip, brotli)
3. Static file server (caching headers)
4. Static file server (directory listing)
5. Static file server (range requests)
6. Static file server (head requests)

#### Phase 1C \- Built-in Device(s)

1. Identity device

#### Phase 1D \- Reload

1. Structured logs (logs command)
2. Hot reload (SIGHUP \+ admin)

## Phase 1.5: Benchmark and Revisit Lifecycle

*Confirm that the overall architecture is not accidentally bad.*

Known limitations at this stage:

* Instantiate per call (or per device load)
* Store lifetime is local
* No pooling or reuse

Todo:

1. Begin Benchmark suite
2. Evaluate possible performance bottlenecks and make/plan improvements
3. Evaluate sanity of error handling in lifecycle before moving on.

## Phase 2: Load Balancing and Observability(v0.3.x \- v0.4.x)

### Goals

* True, modern load balancing
* Built-in health checks
* **Runtime** service discovery

### Features

* Load balancers:
    * round-robin
    * least-connections
    * randomized
    * pingora-native algorithms

* Health checks:
    * Ping/HTTP checks
    * per-upstream thresholds
    * circuit breaker

* Observability:
    * structured logs
    * request counters
    * upstream timing histograms
    * error metrics

### Deliverables

* Upstream pool manager
* Health-check worker loop
* Runtime updating of upstreams
* Discovery polling with TTL
* `/admin/health`
* `/admin/stats`
* `/admin/reload`

### Implementation Order

#### Phase 2A \- Edge Viability

1. Multiple upstreams (ordered failover)
2. Basic downstream TLS
3. Websocket-proxy support
4. Reload upstreams via `snakeway reload` command
5. Upstream TLS
6. gRPC-proxy support

#### Phase 2B \- Traffic Intelligence

1. Load balancing strategies
2. Health checks
3. Circuit breaking
4. Guard against leaky connection metrics
    - Add RAII-based request guard.
    - Add health-driven circuit open as a secondary signal.

#### Phase 2C \- Observability

1. Observability admin endpoints (building on health checks)

## Phase 2.5: Outstanding and emergent tasks (v0.5.x)

- **Traffic Management: New Weighted Load Balancing**
    - Add a weighted load balancing strategy.
    - Support config-defined weights for A/B testing.
    - Validate weight normalization and edge cases.

- **Config Validation**
    - Audit validation coverage (all sections).
    - Enforce cross-field and reload safety rules.
    - Add invalid-config and reload rejection tests.

- **Config Observability**
    - Add an option to the `config dump` command to format the config hierarchically to better show relationships.

- **Verify all config params are actually used**
    - Audit all config params and ensure they are used.

- **Architecture Review**
    - Review ownership and lifetimes.
    - Audit public traits / hook surfaces.
    - Sanity-check error model.
    - Identify (document) performance footguns.

- **Device Ordering**
    - Define explicit device ordering mechanism (across split config files).
    - Validate ordering conflicts / duplicates.
    - Document ordering semantics.

- **ALPN (Application-Layer Protocol Negotiation)**
    - Review current ALPN behavior (downstream + upstream).
    - Decide explicit policy (http/1.1 vs h2 vs h2c vs grpc).
    - Validate and document protocol negotiation rules.

- **HttpProxy Refactor**
    - Audit HttpProxy implementation.
    - Identify logic to extract into focused components.
    - Reduce HttpProxy to orchestration + wiring.

- **Active Health Checks (Future)**
    - Define a background probe model (HTTP / TCP).
    - Ensure independence from request traffic.
    - Document need for idle-service detection.

- **Routing**
    - Evaluate regex-based path matching
    - Decide support vs non-goal
    - Document matching precedence rules

- **Docs**
    - Update architecture overview
    - Update config reference
    - Add current phase/status snapshot

## Phase 3: Path Control and Security (v0.6.x)

### Goals

* Security filters
* Request normalization
* Abuse prevention

### Features

* Normalization:
    * path collapse
    * UTF-8 enforcement
    * query canonicalization

* Blocking features:
    * CIDR-based allow/deny
    * method allowlist
    * header allow/deny
    * size limits
    * rate limiting

### Deliverables

* Built-in security devices
* Global rate limit device
* Per-route rate limit
* Per-IP behavior tracking

## Phase 4: ACME TLS Automation (v0.7.x)

### Goals

* Let’s Encrypt automation
* Automatic cert renewal
* Integration with devices

### Features

* ACMEv2 support:
    * http-01
    * dns-01 (via device plugin)

* certificate storage
* renewal worker

### Deliverables

* Fully automated TLS
* Cert transparency logs
* `/admin/certs` endpoint

## Phase 5: Architecture and Test Suite (v0.8.x)

*Take a breath and re-evaluate.*

All core features should be implemented at this stage.   
It is a good time to pause and re-evaluate the overall architecture and flesh out the holes in the test suite.

### Goals

* All core features are implemented
* Architecture is clean and forward looking
* Test suite is production grade

### Features

* None

### Deliverables

* A document identifying any gaps in features or architecture.
* Likely 150+ integration tests

## Phase 6: Packaging and Distributions (v0.9.x)

### Goals

* `.deb` packages
* `.rpm` packages
* Systemd service
* Docker distroless images
* Helm chart

### Deliverables

`/etc/snakeway/` directory structure:

```shell
/etc/snakeway/snakeway.toml
/etc/snakeway/routes.d/*.toml
/etc/snakeway/services.d/*.toml
/etc/snakeway/devices.d/*.toml  
```

* Systemd unit
* Debian \+ RPM builders via cargo-deb \+ cargo-rpm
* GitHub releases with proper packaging

## Phase 7: Snakeway 1.0 (v1.0.x)

*Stabilize, package, benchmark, document.*

### Goals

* Comprehensive documentation site
* Full operator manual
* Benchmark suite
* Stabilized plugin API

## Post-v1.0.x

The following is pushed out past the v1.0.x release, because it is not in the critical path.

The additional static file features range from nice-to-haves to critical for a static file server, but static files are
only an ancillary feature of Snakeway.

Similarly, the external control plane and discovery features are only important after the core functionality exists.

### Static file server

1. For large files, server precompressed assets (.br/.gz)
2. Use sendfile for zero-copy serving
3. WASM hooks
4. Per-file caching headers

### External Control Planes and Discovery

1. Dynamic cert management
2. Service discovery:

* DNS A/AAAA \+ TTL
* SRV records
* optional plugin-based discovery
* optional file-based watcher