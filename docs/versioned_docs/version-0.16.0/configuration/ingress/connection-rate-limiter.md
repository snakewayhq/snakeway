---
title: Connection Rate Limiter
---

The `connection_rate_limiting_filter` block applies soft, time-windowed admission control to new incoming connections on a per-client IP basis.
It is nested inside the [bind](bind.md) block.

```hcl
connection_rate_limiting_filter = {
  # Maximum average new connections per second over the window.
  max_connections_per_second = 10

  # Duration of the measurement window in seconds.
  window_seconds             = 3
}
```

## Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `max_connections_per_second` | `integer` | (required) | The maximum allowed average number of new connections per second, calculated over the configured time window. |
| `window_seconds` | `integer` | (required) | The duration of the time window, in seconds, used to measure the connection rate. Shorter windows react faster but allow more burstiness. Longer windows react more slowly but provide smoother enforcement. |

:::note
This is a soft limiter.
Short bursts of traffic may be allowed, but sustained high connection rates will eventually trigger rejections.
:::
