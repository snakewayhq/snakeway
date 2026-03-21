---
title: Entry Point Configuration
---

The `server` and `include` configuration blocks control how Snakeway runs as a process and which devices/ingresses it
loads. It defines how Snakeway starts, and manages its execution environment.

These blocks are located in the config directory under `CONFIG_ROOT/snakeway.hcl`.

```hcl
server {
  version       = 1
  pid_file      = "/var/run/snakeway.pid"
  threads       = 8
  work_stealing = true
  ca_file       = "/path/to/certs/ca.pem"
}

include {
  devices   = "device.d/*.hcl"
  ingresses = "ingress.d/*.hcl"
}
```

Summary:

- `version` the version of the configuration file format.
- `pid_file` enables external process control and supervision.
- `threads` is optional and intended for advanced tuning.
- `work_stealing` is optional and intended for advanced tuning.
- `ca_file` is optional and used to verify upstream certificates.
- `tls_automation` is optional and configures automatic ACME certificate management.
- `include` tells snakeway where to look for device and ingress files.

## Server

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
For most deployments, leaving this option unset is likely not the right choice.
You most likely want to make it match the number of cores on your server.

If `threads` is **not set**, Snakeway does not select a value on your behalf. Instead, it *defers entirely* to the
Pingora runtime's internal defaults.
:::

#### work_stealing

**Type:** `boolean`  
**Default:** `true`

Allow work stealing between threads.

Defaults to `true` - as in work stealing is enabled.

This should be left enabled unless there is a good reason not to.

For example, if you need:

1. Strict CPU affinity.
2. Predictable cache locality.
3. Deterministic performance per worker.

Example scenarios:

1. Extremely latency-sensitive trading workloads.
2. Advanced NUMA pinning setups.
3. Custom per-thread sharding logic.
4. Experimental real-time constraints.

```hcl
server {
  work_stealing = true
}
```

#### ca_file

**Type:** `string`
**Required:** no

Certificate Authority file used to verify upstream certificates.

When an upstream endpoint has TLS configured without its own `ca_file`, this global value is used as a fallback.

```hcl
server {
  ca_file = "/path/to/certs/ca.pem"
}
```

#### tls_automation

**Type:** `object`
**Required:** no

Configures automatic TLS certificate issuance and renewal via the ACME protocol (e.g., Let's Encrypt).

When set, Snakeway will automatically obtain and renew certificates for any `bind` blocks configured with `mode = "acme"`.

See the [TLS Cert Management](/docs/guide/tls-cert-management) guide for full details.

```hcl
server {
  tls_automation = {
    renew_within_days = 30
    acme = {
      directory_url = "https://acme-v02.api.letsencrypt.org/directory"
      data_dir      = "/var/lib/snakeway/acme"
      contact_email = ["admin@example.com"]
    }
    cert_store = {
      type     = "filesystem"
      cert_dir = "/var/lib/snakeway/acme/certs"
    }
  }
}
```

##### renew_within_days

**Type:** `integer`
**Default:** `30`

How many days before expiry to begin attempting renewal. Recommended range: 7–30.

##### acme

**Type:** `object`
**Required:** yes (when `tls_automation` is set)

ACME server connection details.

- `directory_url`: The ACME directory URL (e.g., Let's Encrypt's production or staging endpoint).
- `data_dir`: Directory where ACME account keys and order state are persisted.
- `contact_email`: One or more contact email addresses registered with the ACME provider.
- `ca_file`: Optional CA certificate file for verifying the ACME server's TLS certificate (useful for staging/testing environments).

##### cert_store

**Type:** `object`
**Required:** yes (when `tls_automation` is set)

Determines where issued certificates are stored. Two types are supported:

`filesystem`: persists certificates to disk across restarts (recommended for production):

```hcl
cert_store = {
  type     = "filesystem"
  cert_dir = "/var/lib/snakeway/acme/certs"
}
```

`memory`: stores certificates in memory only; certificates are lost on restart (useful for development and testing):

```hcl
cert_store = {
  type = "memory"
}
```
