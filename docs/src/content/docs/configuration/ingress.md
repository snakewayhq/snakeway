---
title: Server Configuration
---

Ingress files are located in the config directory under `CONFIG_ROOT/ingress.d/*.hcl`.

## Bind

An ingress configuration file defines the ingress rules for a particular bound address.

```hcl
bind = {
  addr         = "127.0.0.1:8443"
  enable_http2 = true

  tls = {
    cert = "/path/to/certs/server.pem"
    key  = "/path/to/certs/server.key"
  }
}
```

## Admin Bind

Snakeway provides a built-in Admin API for observability and operational insight.
These endpoints are available on the `bind_admin` address under the `/admin/` path.

```hcl
bind_admin = {
  addr = "127.0.0.1:8440"
  tls = {
    cert = "/path/to/certs/server.pem"
    key  = "/path/to/certs/server.key"
  }
}
```

:::danger
The Admin API provides significant control over your proxy. Ensure that access is restricted using:

- **network-level firewalls**
- or, by **binding only to trusted interfaces**.

Future versions of Snakeway may include built-in authentication for the Admin API.
:::

## Services

An ingress configuration file may define one or more services:

```hcl
services = [
  {
    load_balancing_strategy = "round_robin"

    health_check = {
      enable                     = false
      failure_threshold          = 3
      unhealthy_cooldown_seconds = 10
    }

    circuit_breaker = {
      enable_auto_recovery       = false
      failure_threshold          = 3
      open_duration_milliseconds = 10000
      half_open_max_requests     = 1
      success_threshold          = 2
      count_http_5xx_as_failure  = false
    }

    routes = [
      {
        path = "/api"
      },
      {
        path               = "/ws"
        enable_websocket   = true
        ws_max_connections = 10000
      }
    ]

    upstreams = [
      {
        weight = 1
        addr   = "127.0.0.1:3443"
      },
      {
        weight = 1
        addr   = "127.0.0.1:3444"
      },
      {
        weight = 1
        sock   = "/tmp/snakeway-http-1.sock"
        sock_options = {
          use_tls = true,
          sni     = "example.com"
        }
      }
    ]
  }
]
```

### Top-level Options

#### Load Balancing Strategy

**Type:** `string`  
**Default:** `failover`

Supported strategies:

- `failover`: Always picks the first healthy upstream in the list.
- `round_robin`: Distributes requests evenly across upstreams.
- `request_pressure`: Picks the upstream with the lowest recent request pressure (heuristic-based, not transport-level).
- `random`: Picks a random healthy upstream.
- `sticky_hash`: Consistent hashing based on request characteristics.

### Routes

##### path

**Type:** `string`  
**Required:** `true`

The URL path prefix to match. Must:

- start with `/`
- be unique across all routes

##### enable_websocket

**Type:** `boolean`  
**Default:** `false`

Enables WebSocket upgrades for this route.

##### ws_max_connections

**Type:** `integer`  
**Optional**

The maximum number of concurrent WebSocket connections allowed for this route.

### Upstreams

Each service can have one or more upstream servers defined. Upstreams represent the backend servers that will handle the
proxied requests.

:::note
Only specify addr or sock, not both. They are mutually exclusive on a single upstream,
but a single service may have mixed upstreams.
:::

#### addr

**Type:** `string`  
**Required:** `false`

The address of the upstream server: host, and port (e.g., `10.0.0.1:8080`).

The protocol is inferred from the `bind` address.

#### sock

**Type:** `string`  
**Required:** `false`

The local filesystem path to a Unix domain socket (e.g., /run/snakeway-http-1.sock).

:::note
The underlying Pingora runtime requires TLS to be configured end-to-end.
This might not be ideal for UDS-based services.
Consider using `addr` instead.
:::

#### weight

**Type:** `integer`  
**Default:** `1`

The weight of this upstream for load balancing strategies that support weighted distribution (i.e., `round_robin`).
Higher weights receive proportionally more traffic.
A weight of `10` will receive approximately 10 times more requests than a weight of `1`.

## Static Files

...
