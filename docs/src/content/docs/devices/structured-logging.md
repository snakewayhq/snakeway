---
title: Structured Logging Device
---


The **Structured Logging device** is a builtin Snakeway device that emits structured tracing events at key points in the
request/response lifecycle.

It is designed to provide **high-signal, low-noise** observability while remaining safe for production and compliant by
default.

:::info
This device always runs last in the device pipeline.
:::

## Design Goals

The Structured Logging device is built around a few core principles:

* Logs should be **structured**, not free-form text
* Sensitive data should be **excluded by default**
* Operators should be able to **opt in** to additional detail
* Logging should integrate cleanly with `tracing` and existing Rust tooling

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

You can control *when* logs are emitted using event and phase filters.

### Events

```hcl
events = ["request", "response"]
```

### Phases

```hcl
phases = ["request", "response"]
```

Phases provide a coarse-grained way to reduce log volume without listing individual hooks.

## Identity-Aware Logging

When used together with the Identity device, Structured Logging can include **EU-safe identity signals**.

Supported identity fields include:

* `country`
* `region`
* `device`
* `bot`
* `asn`

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

## Integration with Tracing

The Structured Logging device emits events using the `tracing` crate.

Output format (JSON vs pretty) and sinks (stdout, files, OpenTelemetry, etc.) are configured **globally** via
`tracing_subscriber`, not in this device.

This keeps logging behavior consistent across Snakeway and application code.
