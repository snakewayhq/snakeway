---
title: Upstreams
---

Each [service](services.md) can have one or more upstream servers.
Upstreams represent the backend servers that handle proxied requests.
Each upstream is either a TCP endpoint or a Unix domain socket.

## TCP Endpoint

```hcl
upstreams = [
  {
    # Weight for load balancing.
    weight   = 1

    # TCP address of the backend.
    endpoint = { host = "127.0.0.1", port = 3443 }
  }
]
```

## Unix Domain Socket

```hcl
upstreams = [
  {
    weight = 1

    # Unix domain socket of the backend.
    sock = { path = "/tmp/snakeway-http-1.sock" }
  }
]
```

## Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `weight` | `integer` | `1` | The weight of this upstream for load balancing strategies that support weighted distribution (e.g., `round_robin`). Higher weights receive proportionally more traffic. |
| `endpoint` | `object` | (optional) | The address of the upstream server. Contains `host` and `port` fields, and an optional `tls` block. See [Upstream TLS](upstream-tls.md). |
| `endpoint.host` | `string` | (required) | The hostname or IP address of the upstream server. |
| `endpoint.port` | `integer` | (required) | The port of the upstream server. |
| `sock` | `object` | (optional) | The Unix domain socket of the upstream server. Contains a `path` field and an optional `tls` block. See [Upstream TLS](upstream-tls.md). |
| `sock.path` | `string` | (required) | The local filesystem path to the Unix domain socket, for example `/run/snakeway-http-1.sock`. |

:::note
Only specify `endpoint` or `sock` on a given upstream, not both.
They are mutually exclusive on a single upstream, but a single service may contain a mix of TCP and socket upstreams.
:::
