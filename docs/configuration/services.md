# Service Configuration

Services define how groups of upstreams are managed, including load balancing and circuit breaking.

## Overview

```toml
[[services]]
name = "api"
strategy = "round_robin"

[[services.upstream]]
url = "http://10.0.0.1:8080"

[[services.upstream]]
url = "http://10.0.0.2:8080"

[services.health_check]
enable = true
failure_threshold = 3
unhealthy_cooldown_seconds = 10

[services.circuit_breaker]
enable_auto_recovery = true
failure_threshold = 5
open_duration_ms = 10000
half_open_max_requests = 1
success_threshold = 2
count_http_5xx_as_failure = true
```

## Load Balancing Strategy

**Type:** `string`  
**Default:** `failover`

Supported strategies:

- `round_robin`: Distributes requests evenly across upstreams.
- `request_pressure`: Picks the upstream with the fewest active requests (this heuristic-based, not transport-level).
- `random`: Picks a random healthy upstream.
- `sticky_hash`: Consistent hashing based on request characteristics.
- `failover`: Always picks the first healthy upstream in the list.

## Circuit Breaker

The circuit breaker protects your services by aggressively stopping traffic to failing upstreams.

### enabled

**Type:** `boolean`  
**Default:** `true`

### failure_threshold

**Type:** `integer`  
**Default:** `5`

Number of consecutive failures (transport errors or 5xx) in the `Closed` state before tripping the circuit to `Open`.

### open_duration_ms

**Type:** `integer`  
**Default:** `10000` (10 seconds)

How long the circuit remains `Open` before transitioning to `HalfOpen` to allow probes.

### half_open_max_requests

**Type:** `integer`  
**Default:** `1`

How many simultaneous probe requests are allowed while in the `HalfOpen` state.

### success_threshold

**Type:** `integer`  
**Default:** `2`

How many successful probes are required in `HalfOpen` to close the circuit again.

### count_http_5xx_as_failure

**Type:** `boolean`  
**Default:** `true`

Whether HTTP 5xx responses from the upstream count as failures for the circuit breaker.
