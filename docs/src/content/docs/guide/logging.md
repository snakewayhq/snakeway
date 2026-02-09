---
title: Logging
---


This page describes how to configure and operate logging at runtime.

---

:::note
For information on structured observability see the
Snakeway [Structured Logging](/configuration/devices/structured-logging/) device.
:::

## Default Behavior

* Log format: JSON
* Log level: `info`
* Output: stdout

## Environment Variables

#### `RUST_LOG`

Controls log filtering.

Examples:

```shell
export RUST_LOG=info
export RUST_LOG=warn,snakeway=info
export RUST_LOG=warn,snakeway=debug,pingora=error 
```

If unset or invalid, logging defaults to:

```text
info
```

#### `SNAKEWAY_LOG_DIR`

Enables file-based logging with daily rotation.

When set:

* Logs are written to `snakeway.log` in the specified directory.
* Logs rotate daily.
* Output is written to files instead of stdout.

Example:

```shell
export SNAKEWAY_LOG_DIR=/var/log/snakeway
```

#### `TOKIO_CONSOLE`

Enables Tokio Console mode for interactive debugging.

When set:

* Normal logging is disabled.
* Tokio Console instrumentation is enabled.
* Intended for interactive debugging only.

Example:

```shell
export TOKIO_CONSOLE=1
```

This is flag is most useful inline:

```shell
TOKIO_CONSOLE=1 snakeway /etc/snakeway/ 
```
