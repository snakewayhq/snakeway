---
title: CLI
---

Snakeway has a set of commands to help operators:

| Command     | Description                                 |
|-------------|---------------------------------------------|
| config      | Inspect configuration                       |
| run         | Run the Snakeway proxy (default)            |
| reload      | Reload a running Snakeway instance (SIGHUP) |
| logs        | Format logs from standard out               |
| wasm-device | Debug a WASM device in isolation            |

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
  - device.d/identity.hcl
  - device.d/structured_logging.hcl

Next steps:
  snakeway config check
  snakeway run
```

This directory structure should now exist:

```shell
/etc/snakeway/snakeway.hcl
/etc/snakeway/ingress.d/*.hcl
/etc/snakeway/device.d/*.hcl  
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

If it fails, you might see something that looks like this:

```shell
configuration validation failed (2 errors, 0 warnings)

/etc/snakeway/device.d/network_policy.hcl
  error: device requires identity device to be present and enabled

/etc/snakeway/device.d/request_rate_limiting.hcl
  error: device requires identity device to be present and enabled
```

For more structure output, use the `JSON` output format:

```shell
snakeway config check /etc/snakeway --format=json
```

Which produces:

```json
{
  "errors": [
    {
      "severity": "Error",
      "message": "device requires identity device to be present and enabled",
      "origin": {
        "file": "/etc/snakeway/device.d/network_policy.hcl",
        "section": "network_policy_device",
        "index": null
      },
      "help": null
    },
    {
      "severity": "Error",
      "message": "device requires identity device to be present and enabled",
      "origin": {
        "file": "/etc/snakeway/device.d/request_rate_limiting.hcl",
        "section": "request_rate_limiting_device",
        "index": null
      },
      "help": null
    }
  ],
  "warnings": []
}
```

## config dump

Dump the configuration to stdout:

```shell
snakeway config dump /etc/snakeway
```

Various formats are support, i.e., `JSON`, `YAML`, and `HCL`.

To dump as `YAML`:

```shell
snakeway config dump /etc/snakeway --format=yaml
```

By default, the output should match the configuration files.
However, the internal runtime representation can also be examined, i.e., the lower level internal primitives used
by Snakeway. This is useful for debugging.

Print the internal representation out as JSON:

```shell
snakeway config dump /etc/snakeway --format=json --repr=runtime
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

## reload

Reloads via the CLI require Snakeway to be started with a [PID file](/configuration/server/#pid_file) (set in
`snakeway.hcl`).

```shell
snakeway reload
```

You will see a message like:

```shell
snakeway reload
Sent SIGHUP to Snakeway (pid 77120)
```

:::note
It is also possible to reload with the [admin API](/guide/admin-api/#post-adminreload).
:::

## logs

The `logs` command formats snakeway's default output.

Without the logs command, you will see raw JSON.

```shell
snakeway
{"timestamp":"2026-02-07T19:04:52.546015Z","level":"INFO","message":"pid file written","pid_file":"/tmp/snakeway.pid","target":"snakeway_core::server::setup"}
{"timestamp":"2026-02-07T19:04:52.547103Z","level":"INFO","message":"Reload loop started","target":"snakeway_core::server::setup"}
{"timestamp":"2026-02-07T19:04:52.547168Z","level":"INFO","message":"Bootstrap starting","log.target":"pingora_core::server","log.module_path":"pingora_core::server","log.file":"/Users/you/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pingora-core-0.7.0/src/server/mod.rs","log.line":431,"target":"pingora_core::server"}
{"timestamp":"2026-02-07T19:04:52.547203Z","level":"INFO","message":"Bootstrap done","log.target":"pingora_core::server","log.module_path":"pingora_core::server","log.file":"/Users/you/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pingora-core-0.7.0/src/server/mod.rs","log.line":451,"target":"pingora_core::server"}
{"timestamp":"2026-02-07T19:04:52.549202Z","level":"INFO","message":"Server starting","log.target":"pingora_core::server","log.module_path":"pingora_core::server","log.file":"/Users/you/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pingora-core-0.7.0/src/server/mod.rs","log.line":492,"target":"pingora_core::server"}
```

The output of snakeway can be piped into something like [jq](https://jqlang.org/tutorial/) to at least format the JSON
in the terminal, but the build `logs` command will show something a little nicer:

```shell
snakeway | snakeway logs
[INFO] pid file written (snakeway_core::server::setup)
[INFO] Reload loop started (snakeway_core::server::setup)
[INFO] Bootstrap starting (pingora_core::server)
[INFO] Bootstrap done (pingora_core::server)
[INFO] Server starting (pingora_core::server)
```

The logs command can also show request stats:

```shell
snakeway | snakeway logs --stats
```

The specific output will be different depending on your config, but something like this should be displayed:

```shell
Snakeway Stats (10s window)
==========================
RPS: 100.0 | events: 10 | 5xx: 0

Latency (window):
0–1ms    ████                  20.0%
2–5ms    █                      0.0%
6–10ms   ████████████          60.0%
11–25ms  ████                  20.0%
26–50ms  █                      0.0%
51–100ms █                      0.0%
101–250ms █                      0.0%
251–500ms █                      0.0%
501–1000ms █                      0.0%
>1000ms  █                      0.0%

Latency p95 ≈ 25ms | p99 ≈ 25ms


Status: 2xx=10 4xx=0 5xx=0

 --------------------- 
Identity: human=7 bot=3 unknown=0
Devices: bot=3 desktop=2 mobile=3 unknown=2
Connection types: Cable/DSL=7 Corporate=2
Countries: AU=2 IE=1 NL=2 RU=3 US=2
ASNs: 13238=3 13335=2 14907=2 32934=3
ASOs:
Cloudflare, Inc.=2
Facebook, Inc.=3
Wikimedia Foundation Inc.=2
YANDEX LLC=3

```

import {Code} from '@astrojs/starlight/components';

## wasm-device exec

The `wasm-device exec` command allows a WASM file to be executed in isolation.

A hook and a request path can be specified to simulate a request.

The supported hooks are `on_request` and `before_proxy`.
"Response" phase hooks will be supported at a later time.

Execute

```shell
snakeway wasm-device exec \
  /etc/snakeway/wasm/my_wasm_device.wasm \
  --hook=on_request \
  --path="/api/foo/bar"
```
