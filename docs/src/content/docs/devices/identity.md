---
title: Identity Device
---

The **Identity** builtin device provides a canonical, request-scoped view of the client making a request. It resolves
client identity **once**, early in the request lifecycle, and exposes it to downstream devices via a typed extension on
the request context.

## Overview

When enabled, the Identity device:

* Resolves the **true client IP** using a trusted-proxy-aware algorithm
* Records the **proxy chain** when traffic passes through known intermediaries
* Optionally enriches requests with **GeoIP data** (country-only by default)
* Optionally parses the **User-Agent** using a configurable engine
* Stores all identity data **internally**, without mutating headers

Identity data is **not logged**, **not forwarded upstream**, and **not exposed** unless explicitly consumed by another
device.

## Execution Order

The Identity device runs **before** all other devices in the pipeline.

Downstream devices (logging, fraud detection, rate limiting) can safely assume identity data is already present in the
request context.

## Request Context Integration

The Identity device inserts a `ClientIdentity` struct into the request context:

```rust
ctx.extensions.insert(ClientIdentity { /* ... */ });
```

This data is accessed later via:

```rust
let identity = ctx.extensions.get::<ClientIdentity>();
```

This approach avoids header re-parsing and provides a single source of truth for client identity.

## ClientIdentity Structure

The identity data attached to each request includes:

* **ip** — resolved client IP address
* **proxy_chain** — ordered list of trusted proxies (if any)
* **geo** — optional geographic information
* **ua** — optional user-agent classification

Some fields may be unused by certain deployments but are part of the stable identity contract.

## Client IP Resolution

Client IP resolution follows strict, defensive rules:

1. The immediate peer must be a **trusted proxy**
2. `X-Forwarded-For` is walked **right-to-left**
3. The first untrusted IP is selected as the client
4. If no untrusted IP is found, the peer IP is used

This prevents spoofing and aligns with industry best practices.

## GeoIP Enrichment

GeoIP support is **opt-in** and **EU-safe by default**.

* Uses MaxMind MMDB databases
* Memory-mapped for performance
* Decoded lazily via path lookups
* Default enrichment: **country code only**

```toml
enable_geoip = true
geoip_db = "./GeoLite2-Country.mmdb"
```

No city-level or personally identifying location data is collected by default.

## User-Agent Parsing

User-Agent parsing is optional and configurable:

```toml
enable_user_agent = true
ua_engine = "woothee"
```

Supported engines balance accuracy and performance. Defensive limits (such as maximum UA length) are enforced to protect
throughput.

## Compliance & Safety

* Identity data is **internal-only**
* No headers are mutated or forwarded
* No logging occurs unless explicitly enabled elsewhere
* Designed to support EU compliance by default

The Identity device focuses on correctness and reuse, not observability.

## When to Use

Use the Identity device whenever:

* Multiple devices need client context
* You want consistent IP / UA handling
* You need compliance-aware enrichment
* You want to avoid header parsing duplication

It forms the foundation for higher-level features without coupling them together.
