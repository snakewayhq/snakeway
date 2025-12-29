# Admin API

Snakeway provides a built-in Admin API for observability and operational insight. These endpoints are available on the
main listener under the `/admin/` path.

## Endpoints

### `GET /admin/health`

### `GET /admin/upstreams`

Returns a detailed view of all configured services and their upstreams, including health status and circuit breaker
state.

**Example Response:**

```json
{
  "services": {
    "api": {
      "127.0.0.1:8080": {
        "health": { "healthy": true },
        "circuit": "closed",
        "active_requests": 0,
        "total_requests": 150,
        "total_successes": 148,
        "total_failures": 2,
        "circuit_params": {
          "enabled": true,
          "failure_threshold": 5,
          "open_duration_ms": 10000,
          "half_open_max_requests": 1,
          "success_threshold": 2,
          "count_http_5xx_as_failure": true
        },
        "circuit_details": {
          "consecutive_failures": 0,
          "opened_at_rfc3339": null,
          "half_open_in_flight": 0,
          "half_open_successes": 0
        }
      }
    }
  }
}
```

### `GET /admin/stats`

Returns aggregated traffic statistics per service.

**Example Response:**

```json
{
  "api": {
    "active_requests": 0,
    "total_failures": 2,
    "total_requests": 150,
    "total_successes": 148
  }
}
```

## Internal Logging

Snakeway logs significant traffic events to standard output (structured as JSON when configured).

### Circuit Breaker Transitions

When an upstream's circuit state changes, a log entry is generated:

```json
{
  "timestamp": "2023-12-29T15:27:00.000Z",
  "level": "INFO",
  "fields": {
    "event": "circuit_transition",
    "service": "api",
    "upstream": "UpstreamId(12345)",
    "from": "closed",
    "to": "open",
    "reason": "failure_threshold_exceeded",
    "failures": 5
  }
}
```

**Common Reasons:**

- `failure_threshold_exceeded`: Too many consecutive failures in Closed state.
- `cooldown_expired`: Transitioning from Open to HalfOpen after the configured duration.
- `half_open_failure`: Any failure while in HalfOpen state immediately re-opens the circuit.
- `success_threshold_reached`: Successful probes in HalfOpen state closed the circuit.
