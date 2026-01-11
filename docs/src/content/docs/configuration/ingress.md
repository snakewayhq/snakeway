---
title: Server Configuration
---

Ingress files are located in the config directory under `CONFIG_ROOT/ingress.d/*.hcl`.

## Bind

An ingress configuration file defines the ingress rules for a particular bound address.

```hcl
bind = {
  addr         = "127.0.0.1:8443"
  enable_http2 = true

  tls = {
    cert = "/path/to/certs/server.pem"
    key  = "/path/to/certs/server.key"
  }
}
```

## Admin Bind

Snakeway provides a built-in Admin API for observability and operational insight.
These endpoints are available on the `bind_admin` address under the `/admin/` path.

```hcl
bind_admin = {
  addr = "127.0.0.1:8440"
  tls = {
    cert = "/path/to/certs/server.pem"
    key  = "/path/to/certs/server.key"
  }
}
```

:::caution
The Admin API provides significant control over your proxy. Ensure that access is restricted using network-level
firewalls or by binding only to trusted interfaces. Future versions of Snakeway may include built-in authentication for
the Admin API.
:::




