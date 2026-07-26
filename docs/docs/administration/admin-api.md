---
title: Admin API
---

Snakeway includes a built-in administrative API for monitoring proxy health, inspecting upstream services, viewing traffic statistics, and triggering configuration reloads.
All endpoints are served on the address configured in `bind_admin` under the `/admin/` path, and all responses are JSON-formatted.

## Configuration

The Admin API listens on the address specified by `bind_admin` in your ingress configuration.
See the [Admin Bind reference](../configuration/ingress/admin-bind.md) for the full set of fields.

A minimal block looks like:

```hcl
bind_admin = {
  interface = "127.0.0.1"
  port      = 8440
  tls = {
    mode = "manual"
    cert = "/etc/snakeway/admin.crt"
    key  = "/etc/snakeway/admin.key"
  }
  auth = {
    bearer = {
      token_file = "/etc/snakeway/admin.tokens"
    }
  }
}
```

## Authentication

Every admin request must present a bearer token in the `Authorization` header:

```shell
curl -H "Authorization: Bearer $TOKEN" https://127.0.0.1:8440/admin/health
```

Requests without an `Authorization` header, with a non-`Bearer` scheme, or with an unknown token receive a `401 Unauthorized` response and a `WWW-Authenticate: Bearer realm="snakeway-admin"` header.

### Token file format

The file referenced by `auth.bearer.token_file` contains one token per line:

```
a9f1c38de4b67029c5d1e97f4a0ebac12d3b8ffc84e1d27a05f6cb9e83d21a04
7b4e19a2c5f8d3046e9b71c8a52f9e1d4c07bfa6e93d1c24b87a90fed362014c
```

Rules:

- Each token must be at least 32 bytes after trimming whitespace.
  Use a high-entropy source such as `openssl rand -hex 32`.
- Blank lines are rejected to avoid masking format mistakes.
- Comment lines (starting with `#`) are rejected.
  The format is intentionally unambiguous.
- Duplicate tokens are reported as warnings, not errors.

The file should be owned by the Snakeway process user and have mode `0600` (or equivalent ACL).
Filesystem permissions are the at-rest control.
Snakeway does not hash tokens.

### Token rotation

Multiple tokens in the file are all accepted concurrently.
The rotation workflow is:

1. Generate a new token and append it to `token_file`.
2. Run `snakeway reload` (or `POST /admin/reload`).
3. Migrate callers to the new token.
4. Remove the old token from `token_file`.
5. Run `snakeway reload` again.

Because both tokens are valid during steps 2 to 4, there is no window where a caller must choose between the old and new token.

### Defense in depth

Authentication is the innermost of three layers:

1. **Reachability** via `bind_admin.interface` (no wildcard binds).
2. **Transport** via mandatory TLS.
3. **Authentication** via bearer token (this section).

Continue to restrict the listener to a trusted interface and configure TLS.
Authentication limits who can call the API **given** they can reach it.
It does not replace network-level restriction.
This mirrors the guidance published by the [Envoy project](https://www.envoyproxy.io/docs/envoy/latest/operations/admin) and [Caddy project](https://caddyserver.com/docs/api) for their own admin interfaces.

## Endpoint Reference

### `GET /admin/health`

Returns the overall health status of the Snakeway instance and its registered upstream services, including per-upstream health checks and circuit breaker state.

```shell
curl http://localhost:8081/admin/health
```

**Example response:**

```json
{
  "status": "healthy"
}
```

**Error responses:**

| Status                    | Meaning                                       |
|---------------------------|-----------------------------------------------|
| `200 OK`                  | Instance is healthy.                          |
| `503 Service Unavailable` | One or more critical upstreams are unhealthy. |

### `GET /admin/upstreams`

Provides a detailed view of all registered upstreams, including their current health status, circuit breaker state, and request counters.

```shell
curl http://localhost:8081/admin/upstreams
```

**Example response:**

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

The `circuit` field reflects the current circuit breaker state: `"closed"` (normal operation), `"open"` (upstream is failing, requests are rejected), or `"half_open"` (probe requests are being sent to test recovery).

### `GET /admin/stats`

Returns aggregated traffic statistics per service, including active connections and cumulative counters.

```shell
curl http://localhost:8081/admin/stats
```

**Example response:**

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

**Metrics per service:**

| Metric            | Description                                                              |
|-------------------|--------------------------------------------------------------------------|
| `active_requests` | Number of requests currently in flight.                                  |
| `total_requests`  | Cumulative requests handled since last restart or reload.                |
| `total_successes` | Requests that resulted in a successful response (typically 2xx and 3xx). |
| `total_failures`  | Requests that resulted in an error (4xx, 5xx, or connection failures).   |

### `POST /admin/reload`

Triggers a hot reload of the Snakeway configuration.
Snakeway validates the new configuration before applying it.
If validation fails, the existing configuration remains active.

```shell
curl -X POST http://localhost:8081/admin/reload
```

**Example response (success):**

```json
{
  "status": "ok",
  "epoch": 3
}
```

The `epoch` field is a version counter that increments with each successful reload.

**Example response (validation failure):**

```json
{
  "status": "error",
  "message": "invalid configuration: unknown field `cidr_alow` in network_policy_device"
}
```

**Error responses:**

| Status            | Meaning                                                                        |
|-------------------|--------------------------------------------------------------------------------|
| `200 OK`          | Reload was successfully initiated.                                             |
| `400 Bad Request` | Configuration validation failed. The response body contains the error message. |

### `GET /admin/certs`

Returns the current certificate inventory.
Only available when TLS automation is configured.
When no certificates are present (or TLS automation is not enabled), the response contains an empty list.

```shell
curl http://localhost:8081/admin/certs
```

**Example response:**

```json
{
  "certs": [
    {
      "id": "a1b2c3d4",
      "domains": [
        "example.com",
        "api.example.com"
      ],
      "issued_at": "2026-03-01T00:00:00Z",
      "not_after": "2026-05-30T00:00:00Z",
      "expires_in_seconds": 3542400,
      "state": "Valid"
    }
  ]
}
```

**Fields per certificate:**

| Field                | Description                                |
|----------------------|--------------------------------------------|
| `id`                 | Internal certificate identifier.           |
| `domains`            | Domain names covered by the certificate.   |
| `issued_at`          | Timestamp when the certificate was issued. |
| `not_after`          | Certificate expiration timestamp.          |
| `expires_in_seconds` | Seconds remaining until expiration.        |
| `state`              | `"Valid"` or `"Expired"`.                  |

## Circuit Breaker Transition Logs

When an upstream's circuit state changes, Snakeway emits a structured log event:

```json
{
  "timestamp": "2026-03-21T15:27:00.000Z",
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

**Transition reasons:**

- `failure_threshold_exceeded`: Too many consecutive failures in Closed state triggered the circuit to open.
- `cooldown_expired`: The configured open duration elapsed, transitioning from Open to HalfOpen.
- `half_open_failure`: A failure during HalfOpen immediately re-opens the circuit.
- `success_threshold_reached`: Enough successful probes in HalfOpen state closed the circuit.

## Using the Admin API for Monitoring

The `/admin/stats` and `/admin/upstreams` endpoints are designed to be polled by monitoring systems.
A simple health check script might look like this:

```shell
# Check overall health
curl -sf http://localhost:8081/admin/health || echo "Snakeway is unhealthy"

# Fetch stats for a Prometheus exporter or custom dashboard
curl -s http://localhost:8081/admin/stats | jq .

# Inspect a specific service's upstream health
curl -s http://localhost:8081/admin/upstreams | jq '.services.api'
```

Stats counters reset on process restart or successful configuration reload.
For long-term trend analysis, export the counters to an external time-series database at a regular polling interval.
