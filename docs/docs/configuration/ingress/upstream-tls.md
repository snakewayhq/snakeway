---
title: Upstream TLS
---

By default, upstream connections use plain HTTP.
To connect to an upstream over TLS, add a `tls` block inside the `endpoint`.
See [Upstreams](upstreams.md) for the parent structure.

```hcl
endpoint = {
  host = "10.0.0.1"
  port = 8443
  tls = {
    # SNI hostname sent during the TLS handshake.
    sni     = "backend.internal"

    # Whether to verify the upstream certificate.
    verify  = true

    # CA certificate for verification.
    ca_file = "/path/to/certs/ca.pem"
  }
}
```

## Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `tls.sni` | `string` | (required) | The SNI hostname sent during the TLS handshake. |
| `tls.verify` | `boolean` | (required) | Whether to verify the upstream's certificate. Set to `false` only in controlled environments. |
| `tls.ca_file` | `string` | (optional) | Path to a CA certificate file used to verify the upstream's certificate. Falls back to the global `server.ca_file` if not set. |
