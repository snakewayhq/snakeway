---
title: Admin API
---


Snakeway includes a built-in administrative API that allows you to monitor the health of your proxy, inspect upstream
services, view performance statistics, and trigger configuration reloads.

### Endpoint Reference

All Admin API endpoints return JSON-formatted responses.

#### `GET /admin/health`

Returns the overall health status of the Snakeway instance and its registered upstream services.

```bash
curl http://localhost:8081/admin/health
```

#### `GET /admin/upstreams`

Provides a detailed view of all registered upstreams, including their current health status and load balancing metrics.

```bash
curl http://localhost:8081/admin/upstreams
```

#### `GET /admin/stats`

Returns real-time performance statistics, including request and response counters, error rates, and active connection
counts per service.

```bash
curl http://localhost:8081/admin/stats
```

#### `POST /admin/reload`

Triggers an immediate hot reload of the Snakeway configuration. The server will validate the new configuration before
applying it.

```bash
curl -X POST http://localhost:8081/admin/reload
```

The response includes the new configuration "epoch" (a version counter) if the reload was successfully initiated.

## Admin Bindings

These endpoints are available on the `bind_admin` address under the `/admin/` path.

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
        "health": {
          "healthy": true
        },
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

## Metrics

Monitoring the performance and health of your proxy is critical for maintaining a reliable service. Snakeway provides
real-time metrics through its Admin API, giving you visibility into traffic patterns and upstream health.

### The Stats API

The primary way to collect metrics from Snakeway is through the `/admin/stats` endpoint. This endpoint provides counters
and gauges for various system activities.

#### Sample Metrics Output

```json
{
  "api": {
    "active_requests": 5,
    "total_failures": 12,
    "total_requests": 1500,
    "total_successes": 1488
  },
  "static": {
    "active_requests": 0,
    "total_failures": 0,
    "total_requests": 450,
    "total_successes": 450
  }
}
```

### Metrics per Service

Stats are grouped by the service name defined in your configuration. For each service, Snakeway tracks:

- **`active_requests`**: The number of requests currently being processed.
- **`total_requests`**: The cumulative number of requests handled by the service since the last restart or reload.
- **`total_successes`**: The number of requests that resulted in a successful response (typically 2xx and 3xx status
  codes).
- **`total_failures`**: The number of requests that resulted in an error (typically 4xx and 5xx status codes, or
  connection failures).
