---
title: Admin API
---


Snakeway includes a built-in administrative API that allows you to monitor the health of your proxy, inspect upstream
services, view performance statistics, and trigger configuration reloads.

### Enabling the Admin API

By default, the Admin API is disabled. To enable it, you must configure a listener with the `enable_admin` flag set to
`true`:

```toml
[[listener]]
addr = "127.0.0.1:8081"
enable_admin = true
```

It is highly recommended to bind the admin listener to a private network or loopback address to prevent unauthorized
access.

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

### Security Considerations

The Admin API provides significant control over your proxy. Ensure that access is restricted using network-level
firewalls or by binding only to trusted interfaces. Future versions of Snakeway may include built-in authentication for
the Admin API.
