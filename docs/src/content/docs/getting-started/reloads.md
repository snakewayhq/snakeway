---
title: Reloads
---


Snakeway supports hot reloads for configuration files. This means that changes to configuration files can be applied
without restarting the Snakeway process.

## via the CLI

```shell
snakeway reload
```

::: info
Reloads via the CLI require Snakeway to be started with a PID file (set in `snakeway.toml`).
:::

## via the Admin API

You must have the admin API enabled in your configuration file (`snakeway.toml`) to use this endpoint.

```shell
curl -X POST https://10.0.0.1:8443/admin/reload
```
