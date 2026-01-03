# Configuration Overview

Snakeway uses a flexible, directory-based configuration model designed for both simplicity and scale. While small
deployments can live in a single file, larger environments can leverage modularity to organize routes, services, and
devices independently.

### The Entry Point: `snakeway.toml`

The heart of Snakeway's configuration is the `snakeway.toml` file. This file defines the global server settings, network
listeners, and instructions for including modular configuration files.

A typical `snakeway.toml` looks like this:

```toml
[server]
version = 1
threads = 4

[[listener]]
addr = "0.0.0.0:8080"

[include]
routes = "routes.d/*.toml"
services = "services.d/*.toml"
devices = "devices.d/*.toml"
```

### Modular Configuration

The `[include]` section allows you to split your configuration into logical parts using glob patterns. This is
particularly useful for managing many routes or services without creating a monolithic, unmanageable file.

- **`routes.d/`**: Define your request mapping and path-based logic here.
- **`services.d/`**: Define your upstream services and their load balancing strategies.
- **`devices.d/`**: Define the [Devices](/devices/overview) that should be active in the request pipeline.

When Snakeway starts (or reloads), it discovers all files matching these patterns, parses them, and merges them into a
single unified runtime configuration.

### Hot Reloading

Snakeway supports zero-downtime configuration reloads. This means you can update your routes, add new services, or
change device settings without dropping active connections.

Reloads can be triggered in two ways:

1. **SIGHUP Signal**: Send a `SIGHUP` signal to the Snakeway process.
2. **Admin API**: If enabled, you can send a `POST` request to the `/admin/reload` endpoint.

Before a reload is applied, Snakeway performs a full semantic validation of the new configuration. If any errors are
found (e.g., a route pointing to a non-existent service), the reload is aborted, the errors are logged, and the server
continues running with the previous, stable configuration.

### Configuration Validation

You can manually validate your configuration directory at any time using the `snakeway config check` command:

```bash
snakeway config check --path ./config
```

This will report any syntax errors or logical inconsistencies in your configuration files before you attempt to apply
them to a running server.
