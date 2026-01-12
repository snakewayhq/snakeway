---
title: Server Configuration
---

The `server` configuration block controls how Snakeway runs as a process.
It defines how Snakeway starts, and manages its execution environment.

It is located in the config directory under `CONFIG_ROOT/snakeway.hcl`.

```hcl
server {
  version  = 1
  pid_file = "/var/run/snakeway.pid"
  threads  = 8
  ca_file  = "/path/to/certs/ca.pem"
}
```

Summary:

- `version` the version of the configuration file format.
- `pid_file` enables external process control and supervision
- `threads` is optional and intended for advanced tuning
- `ca_file` is optional and used to verify upstream certificates

#### version

**Type:** `integer`  
**Required:** yes

The configuration file format version.

This will be incremented whenever backwards-incompatible changes are made to the configuration file format.
For now this is always `1` and will likely later be incremented per major release (or not at all).

#### pid_file

**Type:** `string`  
**Required:** no

If set, Snakeway will write its process ID (PID) to the specified file on startup.

```hcl
server {
  pid_file = "/var/run/snakeway.pid"
}
```

If the pid file is present, the `snakeway reload` command can be used to reload Snakeway without restarting the process:

This is useful when integrating Snakeway with external tooling such as:

- Process supervisors
- Signal-based reload workflows
- System scripts or orchestration tools

If the PID file cannot be written, Snakeway will log a warning and continue running.

On shutdown, Snakeway attempts to remove the PID file as a best-effort cleanup step.

#### threads

**Type:** `integer`  
**Required:** no

Controls the number of worker threads used by the proxy runtime to process requests.

```hcl
server {
  threads = 8
}
```

:::tip
For most deployments, leaving this option unset is the correct choice.

If `threads` is **not set**, Snakeway does not select a value on your behalf. Instead, it *defers entirely* to the
Pingora runtime's internal defaults.
:::

## ca_file

**Type:** `string`  
**Required:** no

Certificate Authority file used to verify upstream certificates.

This is not optional if upstreams are configured with TLS.

```hcl
server {
  ca_file = "/path/to/certs/ca.pem"
}
```
