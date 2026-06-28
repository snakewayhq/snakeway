---
title: Server Block
---

The `server` block in `snakeway.hcl` controls process-level settings.

## Configuration Example

```hcl
server {
  version = 1                            # Config format version (always 1 for now)
  pid_file = "/var/run/snakeway.pid"      # Optional; enables signal-based reload
  threads = 8                            # Worker threads for the proxy runtime
  ca_file = "/path/to/certs/ca.pem"     # Global CA for verifying upstream TLS
  dns_refresh_interval_seconds = 30       # How often to re-resolve upstream hostnames

  shutdown {
    drain_seconds = 10            # Time for active connections to finish
    force_timeout_seconds = 30            # Hard ceiling on total shutdown time
  }

  upgrade {
    sock        = "/var/run/snakeway_upgrade.sock"
    max_retries = 5
  }

  performance {
    work_stealing = true  # Allow work stealing between threads
    parallel_accepts_per_listener = 1     # Parallel accept tasks per listener
  }

  upstream {
    connection_pool_size = 128      # Idle upstream connections per worker
    connection_timeout_seconds = 10       # Connect timeout (TCP/TLS) - omit to disable
    read_timeout_seconds = 60       # Per-read idle timeout; omit to disable

    source_addresses {
      ipv4 = ["10.0.1.5"]
      ipv6 = ["fd00::1"]
    }
  }
}
```

## Field Reference

`version` integer, **required**. Configuration file format version. Currently always `1`.

`pid_file` string, default: none. Path where Snakeway writes its PID on startup. Enables the `snakeway reload` command.

`threads` integer, default: defers to the Pingora runtime. Number of worker threads for request processing. For most
deployments, set this to the number of CPU cores on your server.

`ca_file` string, default: none. Path to a CA certificate file used to verify upstream TLS connections when no
per-upstream `ca_file` is configured.

`dns_refresh_interval_seconds` integer, default: `30`. How often (in seconds) Snakeway re-resolves
upstream hostnames in the background. When an upstream is configured with a hostname rather than an
IP address, Snakeway resolves it at startup and then periodically refreshes the resolved address on
this interval. This allows DNS changes (e.g., rolling deployments, blue-green switches) to take
effect without a config reload. Valid range: `1` to `3600`.

`tls_automation` object, default: none. Configures automatic ACME certificate issuance and renewal.
See [TLS Automation](tls-automation/index.md).

:::tip
If `threads` is not set, Snakeway defers entirely to the Pingora runtime's internal defaults rather than selecting a
value on your behalf.
:::

## Shutdown

The `shutdown` block controls how Snakeway terminates when it receives a stop signal (SIGTERM).

```hcl
shutdown {
  drain_seconds         = 10
  force_timeout_seconds = 30
}
```

When Snakeway receives SIGTERM, it stops accepting new connections and gives active requests a window
to complete naturally. After the drain window expires, any remaining connections are forcefully dropped.

`shutdown.drain_seconds` integer, default: `10`. How long (in seconds) active connections are
allowed to finish after a shutdown signal. Requests that complete within this window are guaranteed
not to be dropped. Set to `0` to skip the drain phase entirely. Valid range: `0` to `300`.

`shutdown.force_timeout_seconds` integer, default: none. Hard ceiling on the final cleanup step
of graceful shutdown. After this timeout, remaining connections are forcefully terminated regardless
of state. When not set, the cleanup step has no upper bound beyond the drain period. Valid range:
`1` to `300`.

:::caution
If neither field is configured and a long-lived connection (WebSocket, gRPC stream) never closes,
systemd's `TimeoutStopSec` becomes the only backstop. Always configure `drain_seconds` or set
`TimeoutStopSec` in your service unit.
:::

## Upgrade

The `upgrade` block configures zero-drop upgrades, where a new Snakeway process takes over
listener file descriptors from the old process without dropping connections.

```hcl
upgrade {
  sock        = "/var/run/snakeway_upgrade.sock"
  max_retries = 10
}
```

`upgrade.sock` string, default: `/tmp/pingora_upgrade.sock`. Path to the Unix domain socket used
for transferring listener file descriptors between old and new processes during a zero-drop upgrade.
Both processes must agree on this path. Set a unique value when running multiple Snakeway instances
on the same host.

`upgrade.max_retries` integer, default: `5`. Maximum number of retries when connecting to or
accepting on the upgrade socket during FD transfer. Each retry waits one second. Valid range: `1`
to `60`.

See the [Hot Reload internals](../../internals/hot-reload) page for details on how the upgrade
mechanism works.

## Performance

The `performance` block contains runtime tuning knobs that affect throughput and resource usage.

```hcl
performance {
  work_stealing                 = true
  parallel_accepts_per_listener = 4
}
```

`performance.work_stealing` boolean, default: `true`. Controls whether worker threads are allowed
to steal tasks from one another. Enabling work stealing improves throughput under uneven load.

`performance.parallel_accepts_per_listener` integer, default: `1` (Pingora default). Number of
parallel accept tasks spawned per listener file descriptor. Higher values reduce contention under
bursty connection rates (thundering herd). Most deployments do not need to change this. Valid range:
`1` to `64`.

## Upstream

The `upstream` block controls how Snakeway connects to and reads from upstream servers. These are
global defaults applied to every upstream; a future release may add per-service overrides that take
precedence.

```hcl
upstream {
  connection_pool_size       = 256
  connection_timeout_seconds = 10
  read_timeout_seconds       = 60
}
```

`upstream.connection_pool_size` integer, default: `128` (Pingora default). Number of idle keepalive
connections maintained per worker thread to upstream servers. Increase this for high-traffic
deployments with many backends; decrease it on memory-constrained hosts. Valid range: `1` to `65535`.

`upstream.connection_timeout_seconds` integer, optional. How long to wait when establishing a
connection to an upstream (TCP, and the TLS handshake for TLS upstreams) before failing. **Omit the
key to disable it.** When set, the valid range is `1` to `3600`.

`upstream.read_timeout_seconds` integer, optional. Per-read (idle) timeout while reading an upstream
response: if no data arrives within the window the request fails. This bounds a stalled or wedged
origin without breaking slow-but-progressing or streaming responses (the timer resets on every read).
It is **not** applied to websocket upgrades, so idle long-lived websocket connections are not torn
down. **Omit the key to disable it.** When set, the valid range is `1` to `3600`.

### Source Addresses

The `source_addresses` sub-block controls which local IP addresses Snakeway uses as the source when
making outbound connections to upstreams.

```hcl
upstream {
  source_addresses {
    ipv4 = ["10.0.1.5", "10.0.1.6"]
    ipv6 = ["fd00::1"]
  }
}
```

By default, the operating system selects the source address based on its routing table. Use this
block when you need to pin outbound traffic to specific interfaces or source IPs.

`upstream.source_addresses.ipv4` list of strings, default: empty. IPv4 addresses to bind outbound
upstream connections to. When multiple addresses are specified, Snakeway round-robins across them.

`upstream.source_addresses.ipv6` list of strings, default: empty. IPv6 addresses to bind outbound
upstream connections to. When multiple addresses are specified, Snakeway round-robins across them.

Common use cases:

- **Multi-homed hosts** — force upstream traffic through a specific network interface
- **Upstream IP allowlists** — guarantee which source IP upstreams see
- **Port exhaustion** — spread connections across multiple source IPs to get more ephemeral ports

## Observability

The `observability` block configures telemetry export.
It is optional; when omitted, no telemetry data is exported.

### Configuration Example

```hcl
server {
  # ... other fields ...

  observability {
    otel {
      enable         = true
      endpoint       = "http://localhost:4317"
      service_name   = "my-proxy"
      sampling_ratio = 0.1  # sample 10% of root traces
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

`observability.otel.sampling_ratio` float, default: `1.0`. Controls
what fraction of root traces are sampled (0.0 to 1.0). When an incoming
request carries a sampled W3C Trace Context, Snakeway always honors
that decision. For requests without a parent context, this ratio
determines sampling probability. Set to `0.1` to sample 10% of root
traces, or `1.0` to sample all.

## PID File and Reload Behavior

When `pid_file` is set, Snakeway writes its process ID to the specified path at startup. This enables the
`snakeway reload` command, which sends a signal to the running process to reload configuration without a full restart.

The PID file is useful when integrating Snakeway with:

- Process supervisors
- Signal-based reload workflows
- System scripts or orchestration tools

If the PID file cannot be written, Snakeway logs a warning and continues running. On shutdown, Snakeway removes the PID
file as a best-effort cleanup step.
