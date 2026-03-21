---
title: Request Filter Device
---

The **Request Filter device** is a builtin Snakeway device that enforces **cheap, deterministic request gating** rules.

## Configuration Example

This example configures a specific set of rules to reject invalid requests.

```hcl
request_filter_device = {
  enable = true

  allow_methods = ["GET", "POST"]
  deny_methods = ["TRACE"]

  required_headers = ["host"]
  deny_headers = ["x-forwarded-host"]

  max_header_bytes = 16384
  max_body_bytes   = 1048576

  client_body_timeout_seconds = 10

  deny_status = 403
}
```

## Method Filtering

You can explicitly allow or deny HTTP methods.

```hcl
allow_methods = ["GET", "POST"]
deny_methods = ["TRACE", "CONNECT"]
```

Method filtering rules:

* If a method appears in `deny_methods`, the request is rejected
* If `allow_methods` is non-empty, only those methods are allowed
* Deny rules always take precedence over allow rules

## Header Filtering

The Request Filter device supports **three distinct header policies** that work together.

### Required Headers

```hcl
required_headers = ["host", "user-agent"]
```

Required headers enforce **presence**:

* Every listed header must appear in the request
* Other headers are unaffected

Missing any required header results in a `400 Bad Request`.

### Denied Headers

```hcl
deny_headers = ["x-forwarded-host", "x-original-url"]
```

Denied headers enforce an explicit **blacklist**:

* If *any* denied header appears, the request is rejected
* All other headers are allowed

This is useful for blocking dangerous or spoofed headers.

### Allowed Headers (Allowlist)

```hcl
allow_headers = ["host", "user-agent", "authorization"]
```

Allowed headers enforce a **restricted header universe**:

* When `allow_headers` is non-empty:
    * **Only** the listed headers may appear in the request
    * Any other header causes the request to be rejected
* Presence is **not required**
* Use `required_headers` to enforce mandatory headers

:::note
This can be tricky to reason about, but remember:

`allow_headers` controls *which headers may exist*.  
`required_headers` controls *which headers must exist*.

They serve different purposes and are often used together.
:::

### Header Rule Evaluation Order

Header rules are evaluated in the following order:

1. **Denied headers** (fast fail)
2. **Allowed headers** (universe restriction)
3. **Required headers** (presence check)

## Request Size Limits

### Header Size Limit

```hcl
max_header_bytes = 16384  # 16 KB
```

The total serialized size of all request headers is computed and enforced during `on_request`.

Requests exceeding this limit are rejected with `431 Request Header Fields Too Large`.

### Body Size Limits

```hcl
max_body_bytes = 1048576  # 1 MB
max_suspicious_body_bytes = 8192     # 8 KB
```

Request bodies are validated **incrementally** as they are streamed.

Body size limits depend on HTTP method semantics:

* Methods with defined body semantics (`POST`, `PUT`, `PATCH`) use `max_body_bytes`
* Methods where a body is suspicious (`DELETE`, `OPTIONS`) use `max_suspicious_body_bytes`
* Methods that forbid bodies (`GET`, `HEAD`, `TRACE`) may be rejected earlier

If a body exceeds its configured limit, the request is rejected with `413 Payload Too Large`.

## Client Body Timeout

```hcl
client_body_timeout_seconds = 10
```

Controls how long the proxy waits for each chunk of request body data from the client.
If the client stalls mid-body for longer than this duration, the connection is terminated.

This prevents **slowloris-style attacks** where an attacker sends a large `Content-Length`
but trickles body bytes to hold upstream connections open indefinitely.

* Default: not set (Pingora's built-in default of 60 seconds applies)
* Set to a lower value (e.g., `5`–`10`) for public-facing deployments
* The timeout is **per-read**, not cumulative: each successful chunk resets the timer

:::caution
Without this setting, a malicious client can declare `Content-Length: 1000000` and send
one byte per minute, tying up an upstream connection for hours. Set this in any
deployment exposed to untrusted clients.
:::

## Custom Deny Status

You can override the default rejection status code:

```hcl
deny_status = 403
```

When set, this status code is used for all denials.
This allows operators to optionally prevent leaking information about specific rules to clients.

Invalid status codes are rejected at configuration load time.
