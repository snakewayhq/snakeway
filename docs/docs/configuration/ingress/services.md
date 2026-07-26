---
title: Services
---

An ingress configuration file may define zero or more services.
Each service groups a set of [routes](routes.md) with one or more [upstreams](upstreams.md) and an optional load balancing strategy.

```hcl
services = [
  {
    # Load balancing strategy for this service.
    load_balancing_strategy = "round_robin"

    # Optional: health checking for upstreams.
    health_check = {
      enable                     = false
      failure_threshold          = 3
      unhealthy_cooldown_seconds = 10
    }

    # Optional: circuit breaker for upstream protection.
    circuit_breaker = { ... }

    # Routes and upstreams are defined within the service.
    routes    = [ ... ]
    upstreams = [ ... ]
  }
]
```

## Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `load_balancing_strategy` | `string` | `"failover"` | The load balancing strategy. See supported strategies below. |
| `health_check.enable` | `boolean` | `true` | Enable health checking for upstreams. |
| `health_check.failure_threshold` | `integer` | `5` | Number of consecutive failures before marking an upstream as unhealthy. |
| `health_check.unhealthy_cooldown_seconds` | `integer` | `10` | Seconds to wait before re-checking an unhealthy upstream. |
| `circuit_breaker` | `object` | (optional) | Circuit breaker configuration. See [Circuit Breaker](circuit-breaker.md). |
| `routes` | `list(object)` | (required) | Route matching rules. See [Routes](routes.md). |
| `upstreams` | `list(object)` | (required) | Backend upstream servers. See [Upstreams](upstreams.md). |

## Load Balancing Strategies

- `failover`: Always picks the first healthy upstream in the list.
- `round_robin`: Distributes requests evenly across upstreams.
  Respects the `weight` field on each upstream.
- `request_pressure`: Picks the upstream with the lowest recent request pressure (heuristic-based, not transport-level).
- `random`: Picks a random healthy upstream.
- `sticky_hash`: Consistent hashing based on request characteristics.
