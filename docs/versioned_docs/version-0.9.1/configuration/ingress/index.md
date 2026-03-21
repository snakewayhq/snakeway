---
title: Ingress Configuration
---

Ingress files live at `CONFIG_ROOT/ingress.d/*.hcl`. Each file defines a listener bound to an interface and port, along with the services, routes, upstreams, and static file policies attached to it.

## Structure

A complete ingress file follows this skeleton:

```hcl
bind = {
  interface = "..."
  port      = ...
  tls       = { ... }

  connection_filter               = { ... }
  connection_rate_limiting_filter = { ... }
}

bind_admin = { ... }

services = [
  {
    load_balancing_strategy = "..."
    circuit_breaker         = { ... }
    routes                  = [ ... ]
    upstreams               = [ ... ]
  }
]

static_files = [
  {
    routes = [
      {
        hosts         = [...]
        path          = "..."
        file_dir      = "..."
        compression   = { ... }
        cache_policy  = { ... }
      }
    ]
  }
]
```

## Sections

| Section | Description |
|---|---|
| [Bind](bind.md) | Primary listener address, TLS, and protocol settings. |
| [Connection Filter](connection-filter.md) | IP-based access control for incoming connections. |
| [Connection Rate Limiter](connection-rate-limiter.md) | Time-windowed admission control per client IP. |
| [Admin Bind](admin-bind.md) | Admin API listener for observability and operations. |
| [Services](services.md) | Service definitions grouping routes with upstreams. |
| [Circuit Breaker](circuit-breaker.md) | Upstream protection through failure-aware traffic gating. |
| [Routes](routes.md) | Hostname and path-prefix matching rules. |
| [Upstreams](upstreams.md) | Backend servers (TCP endpoints or Unix sockets). |
| [Upstream TLS](upstream-tls.md) | TLS settings for upstream connections. |
| [Static Files](static-files.md) | Serve files directly from the local filesystem. |
| [Compression](compression.md) | Compression settings for static file responses. |
| [Cache Policy](cache-policy.md) | Cache-Control header configuration for static files. |

Both services and static routes use **longest-prefix matching**, so more specific routes take precedence over broader ones.
