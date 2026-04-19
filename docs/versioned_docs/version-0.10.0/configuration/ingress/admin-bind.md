---
title: Admin Bind
---

Snakeway provides a built-in Admin API for observability and operational insight. These endpoints are available on the `bind_admin` address under the `/admin/` path.

```hcl
bind_admin = {
  # The network interface for the admin API.
  interface = "127.0.0.1"

  # The port for the admin API.
  port      = 8440

  # Optional: TLS configuration for the admin listener.
  tls = {
    mode = "manual"
    cert = "/path/to/certs/server.pem"
    key  = "/path/to/certs/server.key"
  }
}
```

## Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `interface` | `string` | (required) | The network interface to bind the admin API to. |
| `port` | `integer` | (required) | The port to bind the admin API to. |
| `tls.mode` | `string` | (optional) | TLS mode. Use `"manual"` to provide your own certificate and key. |
| `tls.cert` | `string` | (optional) | Path to the TLS certificate file. |
| `tls.key` | `string` | (optional) | Path to the TLS private key file. |

:::danger
The Admin API provides significant control over your proxy. Ensure that access is restricted using:

- **network-level firewalls**
- or, by **binding only to trusted interfaces**.

Future versions of Snakeway may include built-in authentication for the Admin API.
:::
