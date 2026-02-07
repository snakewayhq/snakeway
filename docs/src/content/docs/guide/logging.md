---
title: Structured Logging
---


Snakeway provides deep, structured observability through its `StructuredLogging` device. Instead of traditional
line-based logs, Snakeway emits rich, JSON-formatted events that are easy to parse and analyze with modern log
management tools.

### Configuring the Logging Device

To enable structured logging, add the `structured_logging_device` configuration to your `devices.d/` directory:

```hcl
structured_logging_device = {
  enable           = true
  level            = "info"
  include_headers  = true
  allowed_headers = ["User-Agent", "X-Request-Id"]
  redacted_headers = ["Authorization", "Cookie"]
  include_identity = true
  identity_fields = ["country", "device", "bot"]
}
```

### Log Format

By default, Snakeway emits logs to `stdout` in JSON format. Each log entry represents a specific event in the
request/response lifecycle.

A typical request log looks like this:

```json
{
  "timestamp": "2024-05-20T10:00:00Z",
  "level": "INFO",
  "event": "request",
  "method": "GET",
  "uri": "/api/users/1",
  "headers": "{\"user-agent\":\"Mozilla/5.0...\", \"x-request-id\":\"abc-123\"}",
  "identity": "{\"country\":\"US\", \"device\":\"desktop\", \"bot\":\"false\"}"
}
```

### Key Fields

- **`event`**: The lifecycle phase of the request (`request`, `before_proxy`, `after_proxy`, `response`).
- **`method`**: The HTTP method (GET, POST, etc.).
- **`uri`**: The request URI.
- **`status`**: The HTTP status code (present in response phases).
- **`headers`**: A JSON string containing the allowed request/response headers.
- **`identity`**: Information extracted by the `Identity` device, such as GeoIP and User-Agent data.

### Filtering and Redaction

To keep your logs clean and secure, Snakeway offers fine-grained control over header logging:

- **`allowed_headers`**: Only log headers explicitly listed in this array. If empty, all headers (except redacted ones)
  are logged.
- **`redacted_headers`**: Any header listed here will have its value replaced with `<redacted>` in the logs. Use this
  for sensitive information like `Authorization` or `Cookie` headers.

### Controlling Log Volume

You can control which phases of the request/response lifecycle are logged using the `phases` and `events` settings. This
is useful for reducing log volume in high-traffic environments:

```hcl
structured_logging_device = {
  phases = ["response"]
  events = ["response"]
}
```
