---
title: Entry Point Configuration
---

The file `snakeway.hcl` is the entry point for all Snakeway configuration. It defines how Snakeway runs as a process, where it finds TLS certificates, and where it discovers the rest of its configuration files.

## Structure Overview

```hcl
server {
  version            = 1
  pid_file           = "..."
  threads            = 8
  work_stealing      = true
  ca_file            = "..."

  tls_automation = {
    renew_within_days = 30

    acme       = { ... }
    cert_store = { ... }
  }
}

include {
  devices   = "device.d/*.hcl"
  ingresses = "ingress.d/*.hcl"
}
```

## Blocks

### `server`

Process-level settings: worker threads, PID file, TLS automation, and upstream CA configuration.

See [Server Block](server.md).

### `server.tls_automation`

Automatic certificate issuance and renewal via the ACME protocol.

See [TLS Automation](tls-automation/index.md).

### `include`

Glob patterns that tell Snakeway where to find device and ingress configuration files, relative to `CONFIG_ROOT`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `devices` | string | **required** | Glob pattern for device configuration files. |
| `ingresses` | string | **required** | Glob pattern for ingress configuration files. |
