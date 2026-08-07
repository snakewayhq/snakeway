---
title: Structured Logging Device
---


The **Structured Logging device** is a builtin Snakeway device that emits structured tracing events at key points in the request/response lifecycle.

## Configuration Example

```hcl
structured_logging_device = {
  enable           = true
  level            = "info"
  include_identity = true
  identity_fields = ["country", "device"]
  include_headers  = false
  events = ["request", "response"]
}
```

## Field Reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable` | boolean | `false` | Enable or disable the device. |
| `level` | string | `"info"` | Log level: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`. |
| `include_identity` | boolean | `false` | Include resolved identity fields in log events. |
| `identity_fields` | list of strings | `[]` | Which identity fields to include (see below). |
| `include_headers` | boolean | `false` | Include request headers in log events. |
| `allowed_headers` | list of strings | `[]` | Headers to include when `include_headers` is true. |
| `redacted_headers` | list of strings | `[]` | Headers to redact (value replaced, key preserved). |
| `events` | list of strings | all | Which lifecycle events to log. |
| `phases` | list of strings | all | Which phases to log. |

## What Gets Logged

Depending on configuration, the device can emit events for:

* Request arrival (`on_request`)
* Request body streamed (`on_stream_request_body`)
* Before proxying upstream (`before_proxy`)
* After upstream response (`after_proxy`)
* Final response (`on_response`)

Each log event may include:

* HTTP method
* Request URI
* Response status
* Selected identity fields (optional)
* Selected headers (optional)

## Lifecycle Events

When logs are emitted can be controlled using `event` and `phase` filters.

### Events

```hcl
events = ["request", "response"]
```

### Phases

```hcl
phases = ["request", "response"]
```

## Identity-Aware Logging

When used together with the Identity device, Structured Logging can include **identity signals**.

All identity field options:

```hcl
include_identity = true

identity_fields = [
  "asn", # Identifies the client's network (ISP, cloud provider, enterprise)
  "aso", # Human-readable owner of the ASN (e.g. AWS, Comcast)
  "country", # ISO country code (coarse geolocation, privacy-safe)
  "region", # Sub-country region/state (used for traffic analysis and policy)
  "device", # Client device category (desktop, mobile, tablet, bot, etc.)
  "bot", # Flag indicating likely automated traffic vs human
  "connection_type", # Network type used by the client (Cable/DSL, Cellular, Corporate, or Satellite)
  "client_ip", # Resolved client IP address (after XFF/trusted proxy resolution)
  "proxy_chain", # Ordered list of proxy IPs traversed before reaching Snakeway
  "forwarded", # Whether the request arrived via a forwarded (XFF) header
  "trusted", # Whether the forwarded identity was verified against trusted proxies
]
```

Identity logging is:

* Disabled by default
* Field-selectable
* Derived from `ClientIdentity` stored in `ctx.extensions`

This avoids reparsing headers and ensures consistency with Identity resolution.

## Header Logging

:::caution
Headers often contain personal or sensitive data.
Enable this only when necessary.
:::

HTTP headers are **not logged by default**.

If enabled, you can:

* Whitelist which headers are allowed
* Redact sensitive headers by name

```hcl
include_headers = true

allowed_headers = [
  "host",
  "user-agent",
  "x-request-id"
]

redact_headers = [
  "authorization",
  "cookie"
]
```
