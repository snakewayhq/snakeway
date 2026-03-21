---
title: Bind
---

The `bind` block configures the primary listener address and its associated protocol settings.

```hcl
bind = {
  # The network interface to listen on.
  interface    = "127.0.0.1"

  # The port to listen on.
  port         = 8443

  # Enable HTTP/2 support (requires TLS).
  enable_http2 = true

  # Redirect plain HTTP requests to HTTPS.
  redirect_http_to_https = false

  # TLS configuration for the listener.
  tls = {
    mode = "manual"
    cert = "/path/to/certs/server.pem"
    key  = "/path/to/certs/server.key"
  }

  # See: Connection Filter, Connection Rate Limiter
  connection_filter               = { ... }
  connection_rate_limiting_filter = { ... }
}
```

## Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `interface` | `string` | (required) | The network interface to bind to. |
| `port` | `integer` | (required) | The port to bind to on the specified interface. |
| `enable_http2` | `boolean` | `false` | Enable HTTP/2 on the ingress instance. TLS is required when this is enabled. |
| `redirect_http_to_https` | `boolean` | `false` | When enabled, plain HTTP requests are redirected to HTTPS. |
| `tls.mode` | `string` | (required) | TLS mode. Use `"manual"` to provide your own certificate and key. |
| `tls.cert` | `string` | (required) | Path to the TLS certificate file. |
| `tls.key` | `string` | (required) | Path to the TLS private key file. |
| `connection_filter` | `object` | (optional) | Connection filtering rules. See [Connection Filter](connection-filter.md). |
| `connection_rate_limiting_filter` | `object` | (optional) | Connection rate limiting rules. See [Connection Rate Limiter](connection-rate-limiter.md). |

For details on TLS configuration, see the TLS section of the documentation.
