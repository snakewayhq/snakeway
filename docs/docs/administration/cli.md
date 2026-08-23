---
title: CLI
---

You operate Snakeway from the command line.
The `snakeway` binary generates and inspects configuration, runs the proxy, reloads or upgrades a running instance, and formats logs.

The available commands:

| Command     | Description                                 |
|-------------|---------------------------------------------|
| config      | Inspect configuration                       |
| route       | Route debugging tools                       |
| run         | Run the Snakeway proxy (default)            |
| reload      | Reload a running Snakeway instance (SIGHUP) |
| upgrade     | Trigger a zero-drop upgrade (SIGQUIT)       |
| logs        | Format logs from standard out               |
| lsp         | Serve the config language server over stdio |
| wasm-device | Debug a WASM device in isolation            |

## config init

Snakeway cannot run without configuration files.

You can generate a new configuration directory in the current directory:

```shell
snakeway config init
```

Or with a custom directory path and template:

```shell
snakeway config init /etc/snakeway --template=httpbin
```

This produces:

```shell
✔ Initialized Snakeway config in /etc/snakeway
✔ Created:
  - /etc/snakeway/device.d/identity.hcl
  - /etc/snakeway/ingress.d/minimal.hcl
  - /etc/snakeway/snakeway.hcl

Next steps:
  snakeway config check /etc/snakeway
  snakeway run --config /etc/snakeway
```

The init command uses the `minimal` template by default.

Other templates are available.

| Template | Description                                                   |
|----------|---------------------------------------------------------------|
| minimal  | A barebones starting point                                    |
| httpbin  | A simple test configuration to verify the proxy is functional |
| dev      | Used for internal development                                 |

## config check

Before running the proxy, check that the configuration is valid:

```shell
snakeway config check /etc/snakeway
```

If the configuration is valid, the command reports a summary:

```shell
✔ Config loaded successfully
✔ 3 routes
✔ 1 services
✔ 1 upstreams
✔ 2 devices enabled
```

If validation fails, the command lists each error:

```shell
configuration validation failed (2 errors, 0 warnings)

/etc/snakeway/device.d/network_policy.hcl
  error: device requires identity device to be present and enabled

/etc/snakeway/device.d/request_rate_limiting.hcl
  error: device requires identity device to be present and enabled
```

For structured output, use the `JSON` format:

```shell
snakeway config check /etc/snakeway --format=json
```

This produces:

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

The command supports several formats: `JSON`, `YAML`, and `HCL`.

To dump as `YAML`:

```shell
snakeway config dump /etc/snakeway --format=yaml
```

By default (`--repr=spec`), the output matches the configuration files: settings you did not write are omitted, so the dump round-trips back to your source.

Three representations are available through `--repr`:

- `spec` (default): the configuration exactly as written.
- `populated-spec`: the spec with defaults filled in, so you can see the effective values Snakeway applies for settings you omitted.
- `runtime`: the lower level internal primitives Snakeway runs on, after lowering.
  This is useful for debugging.

Show the effective defaults Snakeway will apply:

```shell
snakeway config dump /etc/snakeway --format=hcl --repr=populated-spec
```

Print the internal representation out as JSON:

```shell
snakeway config dump /etc/snakeway --format=json --repr=runtime
```

## route solve

The `route solve` command resolves a URL through the routing table without starting a server.
It uses the exact same config loading, lowering, and routing code as the running proxy, making it ideal for debugging routing issues.

```shell
snakeway route solve http://example.com/api/v1/users --config /etc/snakeway
```

Example output (pretty format):

```shell
Route Solve Result:
  status:    RESOLVED
  route:     service:/api:my-api-service
  kind:      service
  service:   my-api-service
  upstream:  10.0.0.1:8080
```

### Options

| Option        | Default  | Description                                         |
|---------------|----------|-----------------------------------------------------|
| `--config`    | `config` | Path to config directory                            |
| `--method`    | `GET`    | HTTP method                                         |
| `--header`    | (none)   | Request header (repeatable, format: `KEY:VALUE`)    |
| `--client-ip` | (none)   | Client IP for policy evaluation                     |
| `--scheme`    | from URL | Override URL scheme (`http` or `https`)             |
| `--path`      | from URL | Override URL path                                   |
| `--query`     | from URL | Override URL query string                           |
| `--body-size` | `0`      | Simulated body size in bytes                        |
| `--lb-key`    | (none)   | Deterministic key for hash-based upstream selection |
| `--lb-index`  | (none)   | Force upstream index selection                      |
| `--format`    | `pretty` | Output format: `pretty` or `json`                   |
| `--trace`     | `false`  | Include evaluation trace steps                      |
| `--verbose`   | `false`  | Verbose output (implies `--trace`)                  |

### JSON output

For machine-readable output, use `--format=json`:

```shell
snakeway route solve http://example.com/api/v1 --config /etc/snakeway --format=json
```

```json
{
  "matched_route": "service:/api:my-api-service",
  "route_kind": "service",
  "upstream_service": "my-api-service",
  "selected_upstream": "10.0.0.1:8080",
  "static_file_dir": null,
  "rejection": null,
  "normalized": {
    "scheme": "http",
    "host": "example.com",
    "method": "GET",
    "path": "/api/v1",
    "query": null,
    "client_ip": null,
    "body_size": 0
  }
}
```

### Deterministic upstream selection

Upstream selection is fully deterministic and does not use randomness or clock-based logic.

- **`--lb-index N`**: selects the upstream at index `N % upstream_count`
- **`--lb-key STRING`**: hashes the key with FNV-1a and selects `hash % upstream_count`
- **Default**: always selects index 0

`--lb-index` takes precedence over `--lb-key`.

### Exit codes

| Code | Meaning                   |
|------|---------------------------|
| `0`  | Resolved (upstream found) |
| `1`  | Invalid CLI input         |
| `2`  | Config load/parse failure |
| `3`  | No route matched          |
| `4`  | Rejected by policy        |

### Debugging workflow

A recommended workflow for diagnosing routing issues:

```shell
# 1. Validate config first
snakeway config check /etc/snakeway

# 2. Test basic route resolution
snakeway route solve http://example.com/api/v1 --config /etc/snakeway

# 3. Trace the full evaluation path
snakeway route solve http://example.com/api/v1 --config /etc/snakeway --trace

# 4. Verbose output with normalized request details
snakeway route solve http://example.com/api/v1 --config /etc/snakeway --verbose

# 5. Test specific upstream selection
snakeway route solve http://example.com/api/v1 --config /etc/snakeway --lb-index 1

# 6. Machine-readable output for scripting
snakeway route solve http://example.com/api/v1 --config /etc/snakeway --format=json
```

## run

Start snakeway:

```shell
snakeway run
```

or with no arguments:

```shell
snakeway
```

You can target a specific config directory:

```shell
snakeway run --config /etc/snakeway
```

### Options

| Option      | Default  | Description                                                   |
|-------------|----------|---------------------------------------------------------------|
| `--config`  | `config` | Path to the Snakeway config directory                         |
| `--upgrade` | `false`  | Start in upgrade mode (receive listener FDs from old process) |
| `--test`    | `false`  | Validate configuration and exit without starting              |

`--test` loads and validates the configuration, then exits with code 0 if valid or 1 if not.
This is useful for verifying that a new binary or config will pass validation before committing to an upgrade.

`--upgrade` is not intended to be used directly by operators.
It is used internally during a zero-drop upgrade to start the new process in FD-receive mode.
See [`upgrade`](#upgrade) below.

### SNAKEWAY_CONFIG environment variable

The `SNAKEWAY_CONFIG` environment variable sets the config directory for all commands.
This avoids repeating `--config` on every invocation when working with a non-default config path (e.g. `/etc/snakeway` in production).

```shell
export SNAKEWAY_CONFIG=/etc/snakeway

# These all use "/etc/snakeway": 
snakeway config check
snakeway config dump
snakeway route solve ...
snakeway run
```

An explicit `--config` flag always takes precedence over the environment variable:

```shell
export SNAKEWAY_CONFIG=/etc/snakeway
snakeway run --config /tmp/debug-config   # uses /tmp/debug-config
```

The packaged systemd unit and Docker image both set this variable to `/etc/snakeway`, so operators who SSH into a production host can run diagnostic commands without specifying the path.

## reload

Reloads via the CLI require Snakeway to be started with a [PID file](/docs/configuration/entry-point/server) (set in `snakeway.hcl`).

```shell
snakeway reload
```

You will see a message like:

```shell
snakeway reload
Sent SIGHUP to Snakeway (pid 77120)
```

:::note
It is also possible to reload with the [admin API](./admin-api.md#post-adminreload).
:::

## upgrade

Triggers a zero-drop upgrade of a running Snakeway instance by sending SIGQUIT.
Like `reload`, this requires a [PID file](/docs/configuration/entry-point/server).

```shell
snakeway upgrade
```

You will see a message like:

```shell
snakeway upgrade
Sent SIGQUIT to Snakeway (pid 77120)
```

This is the manual equivalent of what happens automatically when Snakeway's reload loop detects a listener-level configuration change.
See the [Hot Reload internals](../internals/hot-reload) page for a full explanation of when this is used and what happens during the transition.

:::note
In most cases you do not need to run this command directly.
When `upgrade_sock` is configured and Snakeway detects a listener change during a normal `reload`, it spawns the new process and sends SIGQUIT automatically.
:::

### Options

| Option       | Default             | Description      |
|--------------|---------------------|------------------|
| `--pid-file` | `/tmp/snakeway.pid` | Path to pid file |

### When to use upgrade vs reload

| Change type                          | Command   | Mechanism               |
|--------------------------------------|-----------|-------------------------|
| Routes, services, devices, TLS certs | `reload`  | In-process ArcSwap      |
| Listener address, port, TLS mode     | `upgrade` | Fork/exec + FD transfer |
| Worker threads, work stealing        | `upgrade` | Fork/exec + FD transfer |

## logs

The `logs` command formats snakeway's default output.

Without the logs command, you will see raw JSON.

```shell
snakeway
{"timestamp":"2026-02-07T19:04:52.546015Z","level":"INFO","message":"pid file written","pid_file":"/tmp/snakeway.pid","target":"snakeway::server::setup"}
{"timestamp":"2026-02-07T19:04:52.547103Z","level":"INFO","message":"Reload loop started","target":"snakeway::server::setup"}
{"timestamp":"2026-02-07T19:04:52.547168Z","level":"INFO","message":"Bootstrap starting","log.target":"pingora_core::server","log.module_path":"pingora_core::server","log.file":"/Users/you/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pingora-core-0.7.0/src/server/mod.rs","log.line":431,"target":"pingora_core::server"}
{"timestamp":"2026-02-07T19:04:52.547203Z","level":"INFO","message":"Bootstrap done","log.target":"pingora_core::server","log.module_path":"pingora_core::server","log.file":"/Users/you/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pingora-core-0.7.0/src/server/mod.rs","log.line":451,"target":"pingora_core::server"}
{"timestamp":"2026-02-07T19:04:52.549202Z","level":"INFO","message":"Server starting","log.target":"pingora_core::server","log.module_path":"pingora_core::server","log.file":"/Users/you/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pingora-core-0.7.0/src/server/mod.rs","log.line":492,"target":"pingora_core::server"}
```

The output of snakeway can be piped into something like [jq](https://jqlang.org/tutorial/) to at least format the JSON in the terminal, but the built-in `logs` command formats it more legibly:

```shell
snakeway | snakeway logs
[INFO] pid file written (snakeway::server::setup)
[INFO] Reload loop started (snakeway::server::setup)
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
