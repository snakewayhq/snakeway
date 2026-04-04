---
title: Server Block
---

The `server` block in `snakeway.hcl` controls process-level settings.

## Configuration Example

```hcl
server {
  version = 1                          # Config format version (always 1 for now)
  pid_file = "/var/run/snakeway.pid"    # Optional; enables signal-based reload
  threads = 8                          # Worker threads for the proxy runtime
  work_stealing = true                       # Allow work stealing between threads
  ca_file = "/path/to/certs/ca.pem"    # Global CA for verifying upstream TLS
}
```

## Field Reference

`version` integer, **required**. Configuration file format version. Currently always `1`.

`pid_file` string, default: none. Path where Snakeway writes its PID on startup. Enables the `snakeway reload` command.

`threads` integer, default: defers to the Pingora runtime. Number of worker threads for request processing. For most
deployments, set this to the number of CPU cores on your server.

`work_stealing` boolean, default: `true`. Controls whether worker threads are allowed to steal tasks from one another.
Enabling work stealing improves throughput under uneven load.

`ca_file` string, default: none. Path to a CA certificate file used to verify upstream TLS connections when no
per-upstream `ca_file` is configured.

`tls_automation` object, default: none. Configures automatic ACME certificate issuance and renewal.
See [TLS Automation](tls-automation/index.md).

## Observability

The `observability` block configures telemetry export.
It is optional; when omitted, no telemetry data is exported.

### Configuration Example

```hcl
server {
  # ... other fields ...

  observability {
    otel {
      enable       = true
      endpoint     = "http://localhost:4317"
      service_name = "my-proxy"
      sampling     = "parent_based"
    }
  }
}
```

### Field Reference

`observability` block, optional. Contains telemetry configuration.

`observability.otel` block, optional. Configures OpenTelemetry export.

`observability.otel.enable` boolean, **required**. Enables or disables
the OpenTelemetry exporter.

`observability.otel.endpoint` string, **required** when enabled. gRPC
endpoint for the OTLP exporter (e.g., `http://localhost:4317`).

`observability.otel.service_name` string, **required** when enabled.
Value used for the `service.name` resource attribute in exported
telemetry.

`observability.otel.sampling` string, default: `"parent_based"`.
Sampling strategy. Currently only `"parent_based"` is supported.
Parent-based sampling defers to the incoming trace's sampling decision
when present and samples all root spans.

## PID File and Reload Behavior

When `pid_file` is set, Snakeway writes its process ID to the specified path at startup. This enables the
`snakeway reload` command, which sends a signal to the running process to reload configuration without a full restart.

The PID file is useful when integrating Snakeway with:

- Process supervisors
- Signal-based reload workflows
- System scripts or orchestration tools

If the PID file cannot be written, Snakeway logs a warning and continues running. On shutdown, Snakeway removes the PID
file as a best-effort cleanup step.

:::tip
If `threads` is not set, Snakeway defers entirely to the Pingora runtime's internal defaults rather than selecting a
value on your behalf.
:::
