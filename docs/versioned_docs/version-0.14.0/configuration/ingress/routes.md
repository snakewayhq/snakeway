---
title: Routes
---

Each [service](services.md) contains one or more routes. A route matches incoming requests by hostname and path prefix.

```hcl
routes = [
  {
    # Match requests for these hostnames.
    hosts = ["example.com", "www.example.com"]

    # Match requests with this path prefix.
    path = "/api"
  },
  {
    hosts = ["example.com"]
    path               = "/ws"

    # Enable WebSocket upgrades on this route.
    enable_websocket   = true
    ws_max_connections = 10000
  }
]
```

## Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `hosts` | `list(string)` | (required) | The list of hostnames this route applies to. Requests are only matched if the `Host` header matches one of the specified values. Use `["*"]` to match all hostnames. |
| `path` | `string` | (required) | The URL path prefix to match. Must start with `/` and must be unique across all routes. |
| `enable_websocket` | `boolean` | `false` | Enables WebSocket upgrades for this route. |
| `ws_max_connections` | `integer` | (optional) | The maximum number of concurrent WebSocket connections allowed for this route. |
