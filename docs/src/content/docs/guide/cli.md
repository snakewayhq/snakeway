---
title: CLI
---

Snakeway has a set of commands to help operators:

- config: Inspect configuration
- run: Run the Snakeway proxy (default)
- reload: Reload a running Snakeway instance (SIGHUP)
- logs: Format logs from standard out
- wasm-device: Debug a WASM device in isolation

## config init

Snakeway cannot run without configuration files.

A new configuration directory can be easily generated in the current directory:

```shell
snakeway config init
```

Or, with a custom directory path:

```shell
snakeway config init /etc/snakeway
```

Which will yield...

```shell                                                                                                                  ✔ 
✔ Initialized Snakeway config in /etc/snakeway
✔ Created:
  - snakeway.hcl
  - ingress.d/default.hcl
  - devices.d/identity.hcl
  - devices.d/structured_logging.hcl

Next steps:
  snakeway config check
  snakeway run
```

This directory structure should now exist:

```shell
/etc/snakeway/snakeway.hcl
/etc/snakeway/ingress.d/*.hcl
/etc/snakeway/devices.d/*.hcl  
```

## config check

Ahh, but wait! How to tell if the configuration is valid?

```shell
snakeway config check /etc/snakeway
```

And if everything looks good, something like this will be displayed:

```shell
✔ Config loaded successfully
✔ 3 routes
✔ 1 services
✔ 1 upstreams
✔ 2 devices enabled
```

## config dump

Dump the configuration to stdout:

```shell
snakeway config dump /etc/snakeway
```

Or, as yaml:

```shell
snakeway config dump /etc/snakeway --yaml
```

## run

Start snakeway:

```shell
snakeway run
```

or, simply:

```shell
snakeway
```

A specific config directory can be targeted:

```shell
snakeway run --config /etc/snakeway
```
