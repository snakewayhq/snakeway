---
title: Configuration
---


Snakeway cannot run without configuration files.

Directory structure:

```shell
/etc/snakeway/snakeway.toml
/etc/snakeway/routes.d/*.toml
/etc/snakeway/services.d/*.toml
/etc/snakeway/devices.d/*.toml  
```

The good news?

There are CLI commands to help with configuration!

## Generating a config directory

A new configuration directory can be easily generated:

```shell
snakeway config init
```

Or, with a custom directory:

```shell
snakeway config init /etc/snakeway
```

Which will yield...

```shell                                                                                                                  ✔ 
✔ Initialized Snakeway config in /etc/snakeway
✔ Created:
  - snakeway.toml
  - routes/default.toml
  - services/api.toml
  - devices/identity.toml
  - devices/structured_logging.toml

Next steps:
  snakeway config check
  snakeway run
```

## Validating a config directory

Ahh, but wait! How to tell if the configuration is valid?

```shell
snaekeway config check /etc/snakeway
```

And if everything looks good...

```shell
✔ Config loaded successfully
✔ 3 routes
✔ 1 services
✔ 1 upstreams
✔ 2 devices enabled
```

## Dumping a config directory

Dump the configuration to stdout:

```shell
snakeway config dump /etc/snakeway
```

Or, as yaml:

```shell
snakeway config dump /etc/snakeway --yaml
```
