---
title: Admin Bind
---

Snakeway provides a built-in Admin API for observability and operational insight.
These endpoints are available on the `bind_admin` address under the `/admin/` path.

```hcl
bind_admin = {
  # The network interface for the admin API.
  interface = "127.0.0.1"

  # The port for the admin API.
  port      = 8440

  # TLS configuration for the admin listener. Required.
  tls = {
    mode = "manual"
    cert = "/path/to/certs/server.pem"
    key  = "/path/to/certs/server.key"
  }

  # Authentication. Required.
  auth = {
    bearer = {
      token_file = "/etc/snakeway/admin.tokens"
    }
  }
}
```

## Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `interface` | `string` | (required) | The network interface to bind the admin API to. Loopback or a specific IP. Wildcard binds are rejected. |
| `port` | `integer` | (required) | The port to bind the admin API to. |
| `tls.mode` | `string` | (required) | TLS mode. Must be `"manual"`. ACME is not supported on `bind_admin`. |
| `tls.cert` | `string` | (required) | Path to the TLS certificate file. |
| `tls.key` | `string` | (required) | Path to the TLS private key file. |
| `auth.bearer.token_file` | `string` | (required) | Path to a file containing one or more bearer tokens, one per line. See the [Admin API authentication](../../administration/admin-api.md#authentication) guide. |

## Defense in depth

The Admin API provides significant control over the proxy.
Snakeway enforces three layers on every admin request:

1. **Reachability.**
   `bind_admin` rejects wildcard interfaces (`0.0.0.0`, `::`, `"all"`).
   Operators must bind to loopback or a specific non-public IP.
2. **Transport.**
   TLS is required.
   ACME is not permitted on `bind_admin` because admin certificates should not depend on a public CA.
3. **Authentication.**
   Every request must carry a valid bearer token in the `Authorization` header.
   See [Authentication](../../administration/admin-api.md#authentication) for details on the token file format and the rotation workflow.

:::caution
Authentication is the innermost layer, not a replacement for the other two.
Continue to bind the admin listener to a trusted interface and restrict access at the network level.
:::
