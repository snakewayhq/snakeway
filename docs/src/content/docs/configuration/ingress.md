---
title: Ingress Configuration
---

Ingress files are located in the config directory under `CONFIG_ROOT/ingress.d/*.hcl`.

## Bind

An ingress configuration file defines the ingress rules for a particular bound address.

```hcl
bind = {
  interface    = "127.0.0.1"
  port         = 8443
  enable_http2 = true

  tls = {
    cert = "/path/to/certs/server.pem"
    key  = "/path/to/certs/server.key"
  }

  connection_filter = {
    cidr = {
      allow = []
      deny = [
        "10.0.0.0/8",
        "192.168.0.0/16"
      ]
    }

    ip_family = {
      ipv4 = true
      ipv6 = false
    }

    on_no_peer_addr = "allow" # allow | deny
  }

  connection_rate_limiting_filter = {
    max_connections_per_second = 10
    window_seconds             = 3
  }
}
```

### Top-level Options

#### interface

**Type:** `string`  
**Default:** `none`

This is the interface to bind to.

#### port

**Type:** `integer`  
**Default:** `none`

This is the port to bind to, on the specified interface.

#### enable_http2

**Type:** `boolean`  
**Default:** `false`

Enable HTTP/2 on the ingress instance.
This has ramifications for TLS configuration (TLS is required) and other settings where noted.

### Connection Filter (connection_filter)

Snakeway supports filtering connections based on their source IP address.

#### cidr

**Type:** `object`  
**Default:** `none`

The `cidr` block allows you to specify a list of CIDR blocks to allow or deny connections from.

##### allow

**Type:** `list(string)`  
**Default:** `[]`

CIDR blocks to allow connections from.
If empty, all connections are allowed.

##### deny

**Type:** `list(string)`  
**Default:** `[]`

CIDR blocks to deny connections from.
If empty, no connections are denied.

:::note
The `deny` list takes precedence over the `allow` list.
:::

#### ip_family

**Type:** `object`  
**Default:** `none`

##### ipv4

**Type:** `boolean`  
**Default:** `false`

If `true`, IPv4 connections are allowed.

##### ipv6

**Type:** `boolean`  
**Default:** `false`

If `true`, IPv6 connections are allowed.

:::note
At least one of `ipv4` or `ipv6` must be `true`.

A config error will be reported if both are `false`.
:::

#### on_no_peer_addr

**Type:** `string`  
**Default:** `allow`

Controls the behavior when a connection is made without a peer address.
Default is permissive, i.e., any connection is allowed.

### Rate Limiter (connection_rate_limiting_filter)

Snakeway supports connection rate limiting at the bind level.

This filter applies **soft, time-windowed admission control** to new incoming
connections on a per-client IP basis.

:::note
This is a soft limiter.

Short bursts of traffic may be allowed, but sustained high connection rates will eventually trigger rejections.
:::

#### max_connections_per_second

**Type:** `integer`  
**Default:** `none`

The maximum allowed **average number of new connections per second**,
calculated over the configured time window.

#### window_seconds

**Type:** `integer`  
**Default:** `none`

The duration of the time window, in seconds, used to measure the connection rate.

Shorter windows react faster but allow more burstiness.  
Longer windows react more slowly but provide smoother enforcement.

## Admin Bind

Snakeway provides a built-in Admin API for observability and operational insight.
These endpoints are available on the `bind_admin` address under the `/admin/` path.

```hcl
bind_admin = {
  interface = "127.0.0.1"
  port      = 8440
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

An ingress configuration file may define zero or more services:

### Example

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
        endpoint = { host = "127.0.0.1", port = 3443 }
      },
      {
        weight = 1
        endpoint = { host = "127.0.0.1", port = 3444 }
      },
      {
        weight = 1
        sock   = "/tmp/snakeway-http-1.sock"
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

:::note
For **round_robin**, the weight is specified on the upstream level.
:::

### Circuit Breaker

The circuit breaker protects your services by aggressively stopping traffic to failing upstreams.

#### enable

**Type:** `boolean`  
**Default:** `true`

#### failure_threshold

**Type:** `integer`  
**Default:** `5`

Number of consecutive failures (transport errors or 5xx) in the `Closed` state before tripping the circuit to `Open`.

#### open_duration_milliseconds

**Type:** `integer`  
**Default:** `10000` (10 seconds)

How long the circuit remains `Open` before transitioning to `HalfOpen` to allow probes.

#### half_open_max_requests

**Type:** `integer`  
**Default:** `1`

How many simultaneous probe requests are allowed while in the `HalfOpen` state.

#### success_threshold

**Type:** `integer`  
**Default:** `2`

How many successful probes are required in `HalfOpen` to close the circuit again.

#### count_http_5xx_as_failure

**Type:** `boolean`  
**Default:** `true`

Whether HTTP 5xx responses from the upstream count as failures for the circuit breaker.

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
Only specify endpoint or sock, not both. They are mutually exclusive on a single upstream,
but a single service may have mixed upstreams.
:::

#### endpoint

**Type:** `object`  
**Required:** `false`

The address of the upstream server: host, and port).

Example:

```hcl
endpoint = { host = "10.0.0.1" port = 8080 } 
```

The protocol is inferred from the `bind` block's TLS settings (no settings mean HTTP, TLS means HTTPS).

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

An ingress configuration file may define zero or more static file policies.

### Example

```hcl
static_files = [
  {
    routes = [
      {
        path              = "/assets"
        file_dir          = "/var/www/html"
        index             = "index.html"
        directory_listing = false
        max_file_size = 10485760 // 10 MiB

        compression = {
          enable_gzip     = false
          small_file_threshold = 102400 // 100 KiB
          min_gzip_size = 1024   // 1 KiB
          enable_brotli   = false
          min_brotli_size = 4096
        }

        cache_policy = {
          max_age_seconds = 60
          public          = true
          immutable       = false
        }
      }
    ]
  }
]
```

### Fields

#### path

**Type:** `string`  
**Required:** `true`

The URL path prefix to match.

#### file_dir

**Type:** `string`  
**Required:** `true`

Absolute path to the directory on disk that will be served.

Constraints:

- must be an absolute path
- must exist
- must be a directory
- must not be `/`

#### index

**Type:** `string`  
**Optional**

Filename to serve when a directory is requested.

#### directory_listing

**Type:** `boolean`  
**Default:** `false`

Whether to enable directory listings when no index file is present.

#### max_file_size

**Type:** `integer`  
**Optional**

Maximum file size in bytes. Default: `10485760` (10 MiB)

### Advanced Static Configuration

Static routes include optional configuration for performance and caching.

#### compression

**Type:** `object`  
**Optional**

Advanced configuration for static file handling.

- `small_file_threshold`: (integer) Threshold for small file optimization in bytes. Default: `262144` (256 KiB)
- `min_gzip_size`: (integer) Minimum size to enable gzip compression. Default: `1024` (1 KiB)
- `min_brotli_size`: (integer) Minimum size to enable brotli compression. Default: `4096` (4 KiB)
- `enable_gzip`: (boolean) Enable gzip compression. Default: `true`
- `enable_brotli`: (boolean) Enable brotli compression. Default: `true`

#### cache_policy

**Type:** `object`  
**Optional**

Configuration for the `Cache-Control` header.

- `max_age`: (integer) `max-age` value in seconds. Default: `3600` (1 hour)
- `public`: (boolean) Whether the cache is `public`. Default: `true`
- `immutable`: (boolean) Whether to add the `immutable` directive. Default: `false`

## Operational Notes

**Routing Priority**

Both services and static routes use **longest-prefix matching**, meaning more specific routes take precedence over
broader ones.

