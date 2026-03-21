---
title: Logging
---

Snakeway provides two complementary logging layers: the **Rust tracing framework**, which controls process-level log output, and the **Structured Logging device**, which emits per-request observability events inside the device pipeline. Understanding both layers is essential for effective debugging and production monitoring.

:::note
For per-request structured observability (HTTP method, URI, status, identity fields, and headers), see the [Structured Logging device](/docs/configuration/devices/structured-logging/).
:::

## Rust Tracing Framework

Snakeway uses the Rust `tracing` ecosystem for all process-level logging. By default, logs are emitted as JSON to stdout at the `info` level.

### Default Behavior

- **Format:** JSON
- **Level:** `info`
- **Output:** stdout

A typical log line looks like this:

```json
{
  "timestamp": "2026-03-21T14:32:01.812Z",
  "level": "INFO",
  "target": "snakeway::server",
  "fields": {
    "message": "listening on 0.0.0.0:8080"
  }
}
```

### Log Levels

Snakeway supports the standard tracing levels, from most to least verbose: `trace`, `debug`, `info`, `warn`, `error`. In production, `info` or `warn` is recommended. Use `debug` or `trace` only during active investigation, as they produce substantial output.

## Environment Variables

### `RUST_LOG`

The `RUST_LOG` environment variable controls log filtering using the `tracing-subscriber` `EnvFilter` syntax. You can set a global level, or target specific crates with comma-separated directives.

```shell
# Set everything to info
export RUST_LOG=info

# Warn by default, but show info-level events from Snakeway
export RUST_LOG=warn,snakeway=info

# Suppress noisy Pingora internals while debugging Snakeway
export RUST_LOG=warn,snakeway=debug,pingora=error
```

If `RUST_LOG` is unset or contains an invalid filter, Snakeway defaults to `info`.

### `SNAKEWAY_LOG_DIR`

When set, Snakeway switches from stdout to file-based logging with daily rotation. Logs are written to `snakeway.log` inside the specified directory, and a new file is created each day.

```shell
export SNAKEWAY_LOG_DIR=/var/log/snakeway
```

This is the recommended approach for long-running production deployments where log aggregation pipelines read from disk.

### `TOKIO_CONSOLE`

Setting `TOKIO_CONSOLE` enables Tokio Console instrumentation for interactive async debugging. Normal logging is disabled while this mode is active, so it should only be used during development.

```shell
TOKIO_CONSOLE=1 snakeway run --config /etc/snakeway/
```

## Structured Logging Device

The Rust tracing framework handles process-level events (startup, shutdown, circuit breaker transitions, configuration reloads). For per-request telemetry, enable the [Structured Logging device](/docs/configuration/devices/structured-logging/) in your device configuration. The Structured Logging device emits events at lifecycle hooks such as `on_request` and `on_response`, and can include HTTP metadata, identity fields, and selected headers in each event.

The two layers complement each other: tracing gives you system-level visibility, while the Structured Logging device gives you request-level visibility.

## Troubleshooting

**No log output appears.** Verify that `TOKIO_CONSOLE` is not set. When Tokio Console mode is active, normal log output is suppressed entirely.

**Logs are too verbose.** Narrow the filter with a crate-specific directive. For example, `RUST_LOG=warn,snakeway=info` silences everything except warnings, while still showing Snakeway info-level events.

**Log files are not rotating.** File rotation only activates when `SNAKEWAY_LOG_DIR` is set. Confirm the directory exists and is writable by the Snakeway process.

**Missing request-level detail.** Process-level logs do not include per-request HTTP metadata. Enable the Structured Logging device and configure `events`, `include_identity`, and `include_headers` to capture the fields you need.
