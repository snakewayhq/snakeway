---
title: Server Configuration
---

The `server` configuration block controls how Snakeway runs as a process and how it listens for incoming traffic.

This section focuses on **process-level behavior** and **runtime characteristics**, not routing or request handling. It
defines how Snakeway starts, listens, and manages its execution environment.

## Overview

```toml
[[listener]]
addr = "127.0.0.1:8443"
tls = { cert = "./path/to/certs/server.pem", key = "./path/to/certs/server.key" }
enable_http2 = true
enable_admin = false
```

Summary:

- `addr` defines where Snakeway accepts incoming traffic
- `tls` is optional and enables TLS support
- `enable_http2` is optional and enables HTTP/2 support
- `enable_admin` is optional and enables admin endpoints

## addr

**Type:** `string`  
**Required:** yes

The address Snakeway listens on for incoming connections.

```toml
[[listener]]
addr = "127.0.0.1:8443"
```

This value is passed directly to the underlying listener. Both IP-based and hostname-based bindings are supported.

Common examples:

```toml
addr = "127.0.0.1:8080"   # Local development
```

```toml
listen = "127.0.0.1:443"    # Production
tls = { cert = "./path/to/certs/server.pem", key = "./path/to/certs/server.key" }
```

## tls

**Type:** `object`  
**Required:** no

If set, Snakeway will listen on the specified address using TLS.

```toml
[[listener]]
tls = { cert = "./path/to/certs/server.pem", key = "./path/to/certs/server.key" }
```

## enable_http2

**Type:** `bool`  
**Required:** no

Enables HTTP/2 (e.g., gRPC) support. If enabled, a TLS configuration is required.

```toml
[[listener]]
enable_http2 = true
```

## enable_http2

**Type:** `bool`  
**Required:** no

Enables admin endpoints. If enabled, a TLS configuration is required, and HTTP/2 must be disabled (or omitted).

```toml
[[listener]]
addr = "127.0.0.1:8443"
tls = { cert = "./path/to/certs/server.pem", key = "./path/to/certs/server.key" }
enable_admin = true
```

::: danger
Do not arbitrarily enable admin on a listener.

If enabled, the public gateway will no longer be served from the (now) admin listener.

They are meant to be mutually exclusive.
:::
