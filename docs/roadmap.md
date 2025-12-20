# Snakeway Roadmap

> A modern, extensible reverse proxy built on Pingora 0.6.0

## ✔ Phase 0: Foundation (v0.1.0)

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

* Snakeway is now a proof-of-concept reverse proxy. *

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
    * `POST /admin/reload`

* Observability:
    * structured logs
    * request counters
    * upstream timing histograms
    * error metrics
* Static file server:
    * basic file serving that preempts upstream
    * (possible) features:
        * ETag / If-Modified-Since
        * gzip / brotli
        * range requests
        * directory listing
        * per-file caching headers
        * WASM hooks

### Deliverables

* Complete plugin API draft
* Example devices (header rewrite, logging)
* `/admin/health`
* `/admin/stats`

### Implementation Order

#### ✔ Phase 1A \- Basic Lifecycle Plumbing and Cofnig

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

#### Phase 1D \- Observability and Reload

1. Structured logs (logs command)
2. Hot reload (SIGHUP \+ admin)
3. Observability endpoints

## Phase 1.5 \- Benchmark and Revisit Lifecycle

*Confirm that the overall architecture is not accidentally bad.*

Known limitations at this stage:

* Instantiate per call (or per device load)
* Store lifetime is local
* No pooling or reuse

Todo:

1. Begin Benchmark suite
2. Evaluate possible performance bottlenecks and make/plan improvements
3. Evaluate sanity of error handling in lifecycle before moving on.

## Phase 2: Load Balancing and Discovery (v0.3.x \- v0.5.x)

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

* Service discovery:
    * DNS A/AAAA \+ TTL
    * SRV records
    * optional plugin-based discovery
    * optional file-based watcher

### Deliverables

* Upstream pool manager
* Health-check worker loop
* Runtime updating of upstreams
* Discovery polling with TTL

### Implementation Order

#### Phase 2A \- Edge Viability

1. Multiple upstreams (ordered failover)
2. Basic downstream TLS
3. Websocket support

#### Phase 2B \- Traffic Intelligence

1. Load balancing strategies
2. Health checks
3. Circuit breaking

#### Phase 2C \- Cloud-Native

1. Service discovery
2. Upstream TLS
3. Dynamic cert management

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

*Modern approach to TLS with zero human intervention.*

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

*Snakeway becomes deployable like nginx.*

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
/etc/snakeway/routes.d\*.toml
/etc/snakeway/devices.d\*.toml  
```

* Systemd unit
* Debian \+ RPM builders via cargo-deb \+ cargo-rpm
* GitHub releases with proper packaging

## Phase 7: Snakeway Scripting (v0.10.x)

Snakeway becomes

### Goals

* WASM scripting engine
* Write custom logic for:
    * routing
    * discovery
    * health checks
    * transformations
    * response shaping

### Deliverables

* WASM API (WasmTime)
* Example scripts
* Full WASM sandboxing model

## Phase 8: Snakeway 1.0 (v1.0.x)

*Stabilize, package, benchmark, document.*

### Goals

* Comprehensive documentation site
* Full operator manual
* Benchmark suite
* Stabilized plugin API

## Post-v1.0.x

1. Static file server: WASM hooks
2. Static file server: For lage files, server precompressed assets (.br/.gz)
3. 