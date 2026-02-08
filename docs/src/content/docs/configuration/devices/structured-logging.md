---
title: Structured Logging Device
---


The **Structured Logging device** is a builtin Snakeway device that emits structured tracing events at key points in the
request/response lifecycle.

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
  "asn", # Identifies the client’s network (ISP, cloud provider, enterprise)
  "aso", # Human-readable owner of the ASN (e.g. AWS, Comcast)
  "country", # ISO country code (coarse geolocation, privacy-safe)
  "region", # Sub-country region/state (used for traffic analysis and policy)
  "device", # Client device category (desktop, mobile, tablet, bot, etc.)
  "bot", # Flag indicating likely of automated traffic vs human
  "connection_type", # Network type used by the client (Cable/DSL, Cellular, Corporate, or Satellite)
]
```

Identity logging is:

* Disabled by default
* Field-selectable
* Derived from `ClientIdentity` stored in `ctx.extensions`

This avoids re-parsing headers and ensures consistency with Identity resolution.

## Header Logging

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

> Headers often contain personal or sensitive data. Enable this only when necessary.
