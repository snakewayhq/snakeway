---
title: Bind
---

The `bind` block configures the primary listener address and its associated protocol settings.

```hcl
bind = {
  # The network interface to listen on.
  interface = "127.0.0.1"

  # The port to listen on.
  port = 8443

  # Enable HTTP/2 support (requires TLS).
  # Plaintext upstreams are supported: the proxy translates HTTP/2 to HTTP/1.1 automatically.
  enable_http2 = true

  # Optional HTTP/2 server tuning. Requires enable_http2 = true.
  # All fields are optional; unset fields keep the defaults listed below.
  http2 = {
    max_concurrent_streams         = 100
    max_header_list_size           = 65536
    initial_window_size            = 65535
    initial_connection_window_size = 1048576
  }

  # Redirect plain HTTP requests to HTTPS.
  redirect_http_to_https = false

  # TLS configuration for the listener.
  tls = {
    mode = "manual"
    cert = "/path/to/certs/server.pem"
    key  = "/path/to/certs/server.key"
  }

  # See: Connection Filter, Connection Rate Limiter
  connection_filter = { ... }
  connection_rate_limiting_filter = { ... }
}
```

## Fields

| Field                             | Type      | Default    | Description                                                                                                                                                                                           |
|-----------------------------------|-----------|------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `interface`                       | `string`  | (required) | The network interface to bind to.                                                                                                                                                                     |
| `port`                            | `integer` | (required) | The port to bind to on the specified interface.                                                                                                                                                       |
| `enable_http2`                    | `boolean` | `false`    | Enable HTTP/2 on the listener. TLS is required. When enabled, HTTP/2 clients can connect even if upstream services only speak HTTP/1.1; the proxy translates between the two protocols automatically. |
| `http2`                           | `object`  | (optional) | Advanced HTTP/2 tuning options. See [HTTP/2 settings](#http2-settings).                                                                                                                               |
| `redirect_http_to_https`          | `boolean` | `false`    | When enabled, plain HTTP requests are redirected to HTTPS.                                                                                                                                            |
| `tls.mode`                        | `string`  | (required) | TLS mode. Use `"manual"` to provide your own certificate and key.                                                                                                                                     |
| `tls.cert`                        | `string`  | (required) | Path to the TLS certificate file.                                                                                                                                                                     |
| `tls.key`                         | `string`  | (required) | Path to the TLS private key file.                                                                                                                                                                     |
| `connection_filter`               | `object`  | (optional) | Connection filtering rules. See [Connection Filter](connection-filter.md).                                                                                                                            |
| `connection_rate_limiting_filter` | `object`  | (optional) | Connection rate limiting rules. See [Connection Rate Limiter](connection-rate-limiter.md).                                                                                                            |

For details on TLS configuration, see the TLS section of the documentation.

### HTTP/2 settings

The optional `http2` block tunes the HTTP/2 server parameters advertised to clients.
It is only valid when `enable_http2 = true`. Every field is optional: when the block
(or a field) is omitted, the defaults below apply. Requests that exceed
`max_header_list_size` are refused with status `431`.

| Field                            | Type      | Default | Description                                                                                     |
|----------------------------------|-----------|---------|-------------------------------------------------------------------------------------------------|
| `max_concurrent_streams`         | `integer` | `100`   | Maximum number of simultaneous HTTP/2 streams (in-flight requests) per client connection.       |
| `max_header_list_size`           | `integer` | `65536` | Maximum decoded header list size per request, in bytes.                                         |
| `initial_window_size`            | `integer` | `65535` | Initial flow-control window per stream, in bytes. Must be at most `2147483647`.                 |
| `initial_connection_window_size` | `integer` | `65535` | Connection-level flow-control window for received data, in bytes. Must be at most `2147483647`. |

All values must be greater than zero.
The two window sizes are capped at 2^31 - 1
(2,147,483,647 bytes), the maximum HTTP/2 flow-control window allowed by RFC 9113.

