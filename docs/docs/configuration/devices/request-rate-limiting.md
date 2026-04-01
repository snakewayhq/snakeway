---
title: Request Rate Limiting Device
---

The **Request Rate Limiting device** limits the rate of incoming HTTP requests on a per-client basis using a sliding
window algorithm. It requires the [Identity device](/docs/configuration/devices/identity) to be enabled, because rate
limits are keyed on the resolved client IP address from the `ClientIdentity` extension. The Rate Limiting device runs
after Identity in the device pipeline.

## Configuration Example

This example limits each client to an average of 20 requests per second, measured over a 5-second sliding window.

```hcl
request_rate_limiting_device = {
  enable                  = true
  max_requests_per_second = 20
  window_seconds          = 5
}
```

## Field Reference

| Field                     | Type            | Default | Description                                                                                     |
|---------------------------|-----------------|---------|-------------------------------------------------------------------------------------------------|
| `enable`                  | bool            | `false` | Enables the Rate Limiting device.                                                               |
| `max_requests_per_second` | integer (u16)   | `0`     | The maximum average request rate allowed per client, expressed as requests per second.          |
| `window_seconds`          | integer (u16)   | `0`     | The duration of the sliding window in seconds over which the average rate is calculated.        |
| `paths`                   | list of strings | `[]`    | Path prefixes this device applies to. Empty means all paths. See [Path Scoping](#path-scoping). |

## How the Sliding Window Works

Rather than resetting a counter at fixed intervals (which would allow bursts at interval boundaries), the Rate Limiting
device maintains a sliding window. It counts requests within the most recent `window_seconds` and computes an average
rate. If that average exceeds `max_requests_per_second`, subsequent requests from the same client are rejected with
`429 Too Many Requests` until the rate drops below the threshold.

Short bursts are tolerated as long as the average over the full window stays within the limit. For example, with
`max_requests_per_second = 20` and `window_seconds = 5`, a client could send 40 requests in one second as long as it
sent very few in the preceding four seconds.

## Practical Scenarios

### API Protection

For a public API that should handle normal traffic but reject abusive clients:

```hcl
request_rate_limiting_device = {
  enable                  = true
  max_requests_per_second = 50
  window_seconds          = 10
}
```

A 10-second window provides a stable measurement that smooths out natural traffic variation while still catching
sustained abuse.

### Burst Handling

For services that expect short, legitimate bursts (webhook receivers, batch-upload endpoints), use a longer window to
avoid false positives:

```hcl
request_rate_limiting_device = {
  enable                  = true
  max_requests_per_second = 100
  window_seconds          = 30
}
```

A 30-second window allows high instantaneous throughput as long as the sustained average stays under the limit.

:::tip
Rate limits automatically recover once a client's traffic subsides. There is no manual reset required, and no penalty
period beyond the window duration itself.
:::

## Path Scoping

By default, the Rate Limiting device applies to all requests. The `paths` field restricts enforcement to requests
matching one or more path prefixes.

```hcl
request_rate_limiting_device = {
  enable                  = true
  max_requests_per_second = 20
  window_seconds          = 5
  paths = ["/api/"]
}
```

Path matching uses prefix semantics with slash-boundary awareness: `/api` matches `/api/users` but not `/apikeys`. When
`paths` is empty or omitted, the device applies to all requests.
