---
title: Configuration Overview
---

Snakeway uses a flexible, directory-based configuration model designed for both simplicity and scale.

## The Entry Point

The heart of Snakeway's configuration is the `snakeway.hcl` file.
This file defines the global server settings and modular configuration file locations.

A typical `snakeway.hcl` looks like this:

```hcl
server {
  version  = 1
  pid_file = "/var/run/snakeway.pid"
  ca_file  = "/path/to/certs/ca.pem"
}

include {
  devices = "devices.d/*.hcl"
  ingress = "ingress.d/*.hcl"
}
```

## Modular Configuration

The `include` section allows you to split your configuration into logical parts using glob patterns.

- **`ingress.d/`**: Define your [Ingress](/configuration/ingress) files.
- **`devices.d/`**: Define the [Devices](/devices/overview) that should be active in the request pipeline.

When Snakeway starts (or reloads), it discovers all files matching these patterns, parses them, and merges them into a
single unified runtime configuration. This is discussed in more detail
in [Configuration Internals](/internals/configuration).

## Hot Reloading

Snakeway supports zero-downtime configuration reloads. This means you can update your routes, add new services, or
change device settings without dropping active connections.

Before a reload is applied, Snakeway performs a full semantic validation of the new configuration. If any errors are
found (e.g., a route pointing to a non-existent service), the reload is aborted, the errors are logged, and the server
continues running with the previous, stable configuration.

Reloads can be triggered in two ways:

#### Reload command

Send a `SIGHUP` signal to the Snakeway process.

```bash
snakeway reload
```

#### Admin API

If enabled the admin API is enable, you can send a `POST` request to the `/admin/reload` endpoint.

For example:

```bash
curl -X POST https://127.0.0.1:8440/admin/reload 
```

## Configuration Validation

You can manually validate your configuration directory at any time using the `config check` command:

```bash
snakeway config check --path /etc/snakeway/
```

This will report any syntax errors or logical inconsistencies in your configuration files before you attempt to apply
them to a running server.
