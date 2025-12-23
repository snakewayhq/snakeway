# Server Configuration

The `server` configuration block controls how Snakeway runs as a process and how it listens for incoming traffic.

This section focuses on **process-level behavior** and **runtime characteristics**, not routing or request handling. It
defines how Snakeway starts, listens, and manages its execution environment.

## Overview

```toml
[server]
listen = "0.0.0.0:8080"
pid_file = "/tmp/snakeway.pid"
threads = 8
```

Summary:

- `listen` defines where Snakeway accepts incoming traffic
- `pid_file` enables external process control and supervision
- `threads` is optional and intended for advanced tuning

## listen

**Type:** `string`  
**Required:** yes

The address Snakeway listens on for incoming connections.

```toml
[server]
listen = "0.0.0.0:8080" # [!code focus]
pid_file = "/tmp/snakeway.pid"
threads = 8
```

This value is passed directly to the underlying listener. Both IP-based and hostname-based bindings are supported.

Common examples:

```toml
listen = "127.0.0.1:8080"   # Local development
```

```toml
listen = "127.0.0.1:443"    # Production
```

## pid_file

**Type:** `string`  
**Required:** no

If set, Snakeway will write its process ID (PID) to the specified file on startup.

```toml
[server]
listen = "0.0.0.0:8080"
pid_file = "/tmp/snakeway.pid"  # [!code focus]
threads = 8
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
listen = "0.0.0.0:8080"
pid_file = "/tmp/snakeway.pid"
threads = 8 # [!code focus]
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
