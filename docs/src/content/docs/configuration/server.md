---
title: Server Configuration
---

The `server` configuration block controls how Snakeway runs as a process.

This section focuses on **process-level behavior** and **runtime characteristics**, not routing or request handling. It
defines how Snakeway starts, and manages its execution environment.

## Overview

```toml
[server]
pid_file = "/tmp/snakeway.pid"
threads = 8
ca_file = "./path/to/certs/ca.pem"
```

Summary:

- `pid_file` enables external process control and supervision
- `threads` is optional and intended for advanced tuning
- `ca_file` is optional and used to verify upstream certificates

## pid_file

**Type:** `string`  
**Required:** no

If set, Snakeway will write its process ID (PID) to the specified file on startup.

```toml
[server]
pid_file = "/tmp/snakeway.pid"  # [!code focus]
threads = 8
ca_file = "./path/to/certs/ca.pem"
```

:::
If the pid file is present, the `snakeway reload` command can be used to reload Snakeway without restarting the process:
:::

This is useful when integrating Snakeway with external tooling such as:

- Process supervisors
- Signal-based reload workflows
- System scripts or orchestration tools

If the PID file cannot be written, Snakeway will log a warning and continue running.

On shutdown, Snakeway attempts to remove the PID file as a best-effort cleanup step.

## threads

**Type:** `integer`  
**Required:** no

Controls the number of worker threads used by the proxy runtime to process requests.

```toml
[server]
pid_file = "/tmp/snakeway.pid"
threads = 8 # [!code focus]
ca_file = "./path/to/certs/ca.pem"
```

## ca_file

**Type:** `string`  
**Required:** no

Certificate Authority file used to verify upstream certificates.

This is not optional if upstreams are configured with TLS.

```toml
[server]
pid_file = "/tmp/snakeway.pid"
threads = 8
ca_file = "./path/to/certs/ca.pem" # [!code focus]
```

### Default behavior

If `threads` is **not set**, Snakeway does not select a value on your behalf. Instead, it *defers entirely* to the
runtime’s internal defaults and scheduling heuristics.

This behavior is intentional. The runtime defaults are designed to prioritize:

- Stability
- Predictable latency
- Sensible behavior across platforms

For most deployments, leaving this option unset is the correct choice.

### When to set `threads`

Snakeway does not attempt to infer an optimal thread count. If this value is set, it is assumed to be a deliberate,
environment-specific choice.

You may choose to set `threads` explicitly if:

- You are running on machines with a high core count
- You want consistent CPU utilization across environments
- You have benchmarked and validated a specific worker configuration

## Operational notes

- The control plane (signals, reload handling, configuration management) runs independently of request processing.
- The `threads` setting affects only the request-processing runtimes.
- Changes to `threads` require a process restart; reloads do not resize worker pools.
