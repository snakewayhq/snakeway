---
title: Hot Reload
---

Snakeway supports two distinct reload mechanisms, each suited to a different class of configuration
change. Understanding which mechanism handles which change is important for both operators deploying
config updates and contributors modifying the reload path.

## Two reload paths

### 1. In-process reload (ArcSwap)

Triggered by SIGHUP or the admin API `POST /admin/reload` endpoint. The running process re-reads
the config from disk, builds a new `RuntimeState`, and atomically swaps it into place via
`ArcSwap`. In-flight requests continue using the old state (they hold an `ArcSwap` guard); new
requests pick up the new state immediately.

This path handles changes to:

- Routes (added, removed, modified)
- Services (upstreams, load balancing strategy, circuit breaker, health check)
- Devices (added, removed, reconfigured)
- TLS certificates (ACME rotation, manual cert file changes)
- DNS refresh interval

No connections are dropped. No new process is spawned. The entire operation completes in
microseconds.

### 2. Zero-drop upgrade (fork/exec + FD transfer)

Some configuration fields are baked into the Pingora `Server` and its listener services at
construction time. Changing them requires building a new server, which means a new process. The
zero-drop upgrade path transfers the kernel socket objects from the old process to the new one so
the TCP accept queue is preserved and no connections are lost.

This path handles changes to:

- Listener addresses and ports
- TLS termination mode (none, manual, ACME) or cert/key paths
- HTTP/2 enablement
- Admin listener enablement
- Connection filters (CIDR allow/deny)
- Connection rate limiting filters
- Redirect configuration
- Admin authentication (token file path)
- Worker thread count
- Work stealing

## How the reload loop classifies changes

When a reload is triggered, the reload loop in `ControlPlaneServer` loads the new config from disk
and runs a diff against the currently running config. The diff function
(`classify_config_change` in `runtime/diff.rs`) compares listeners field-by-field and checks the
server-level fields that are baked at construction (`threads`, `work_stealing`).

If only runtime-swappable fields changed, the in-process ArcSwap path runs. If any listener-level
or server-construction field changed, the zero-drop upgrade path runs automatically.

## Zero-drop upgrade sequence

```
1. Reload triggered (SIGHUP or admin API)
2. New config loaded and validated
3. classify_config_change() returns ListenersChanged
4. Old process spawns: snakeway run --config <path> --upgrade
5. New process loads config, builds server and services
6. New process reads old PID from pid_file, sends SIGQUIT
7. Old process receives SIGQUIT
8. Old process serializes listener FDs, sends them over upgrade_sock (SCM_RIGHTS)
9. New process receives FDs via upgrade_sock
10. New process calls server.bootstrap() with received FDs (no bind() needed)
11. New process starts accepting connections on inherited sockets
12. New process binds any new listener addresses that did not exist before
13. Old process stops accepting, drains in-flight requests, exits
```

Steps 6 through 9 are the critical zero-downtime window. Because the kernel socket object is the
same, the listen backlog is preserved. No SYN in the accept queue is refused.

## Key implementation files

| File | Role |
|------|------|
| `snakeway-core/src/runtime/diff.rs` | `classify_config_change()` -- determines ArcSwap vs upgrade |
| `snakeway-core/src/control_plane/server/upgrade.rs` | `spawn_upgrade()` and `signal_old_process()` |
| `snakeway-core/src/control_plane/server/control_plane_server.rs` | Reload loop with diff + dispatch |
| `snakeway-core/src/data_plane/bootstrap.rs` | Passes `Opt { upgrade }` to Pingora, calls `signal_old_process` before `bootstrap()` |
| `snakeway-core/src/runtime/state.rs` | `reload_runtime_state()` -- the ArcSwap path |
| `snakeway-core/src/control_plane/server/reload.rs` | `ReloadHandle` -- SIGHUP signal handler and watch channel |

## Pingora's FD transfer mechanism

Pingora's `transfer_fd` module handles the low-level socket transfer. The `Fds` struct is a
`HashMap<String, RawFd>` keyed by the listener's bind address string (e.g. `0.0.0.0:8080`).

**Sending (old process):** On SIGQUIT, Pingora serializes the map into a space-separated address
list and the corresponding `RawFd` array, then sends both over a Unix domain socket using
`sendmsg` with `SCM_RIGHTS` ancillary data.

**Receiving (new process):** During `bootstrap()`, if `Opt { upgrade: true }`, Pingora creates a
Unix socket at `upgrade_sock`, binds, listens, and accepts a connection. It receives the FDs and
address list via `recvmsg`, then populates the `Fds` table.

**Matching:** When each Pingora service later calls `Listeners::build()`, each
`ListenerEndpointBuilder::listen()` looks up its bind address in the `Fds` table. If found, it
wraps the received FD with `from_raw_fd()` instead of calling `bind()`. If not found (a new
listener that did not exist in the old process), it performs a fresh `bind()`.

Both sides have retry logic. The receiver retries `accept()` up to `upgrade_max_retries` times
with a one-second interval. The sender retries `connect()` on `ENOENT`, `ECONNREFUSED`, and
`EACCES` with the same cadence. This means the SIGQUIT can safely be sent before the new process
has created the socket.

## Platform constraints

The FD transfer mechanism uses `SCM_RIGHTS` via `sendmsg`/`recvmsg`, which is a Linux-specific
code path in Pingora's `transfer_fd` module. On macOS and Windows, the `get_fds_from` and
`send_fds_to` functions are stubs that return errors or no-ops.

Zero-drop upgrades only work on Linux. On other platforms, listener-level changes require a
conventional restart with a brief interruption.

## The upgrade_sock path

Both old and new processes must agree on the `upgrade_sock` path. By default, Pingora uses
`/tmp/pingora_upgrade.sock`. This can be overridden in the server block:

```hcl
server {
  upgrade_sock = "/var/run/snakeway_upgrade.sock"
}
```

Set a unique path when running multiple Snakeway instances on the same host to avoid socket
collisions.

## PID file requirement

The new process sends SIGQUIT to the old process by reading the PID from the configured
`pid_file`. If `pid_file` is not set, the automatic upgrade path cannot determine the old PID and
will fail with an error. The old process continues serving in this case.

## Config diff details

The diff compares listener configs pairwise by position. Two listeners are considered equivalent
when all of the following match:

- `name`
- `addr`
- `tls_termination` (variant, cert path, key path, ACME domains)
- `enable_http2`
- `enable_admin`
- `redirect` (destination, response code)
- `connection_filter` (CIDR lists, IP families, no-peer-addr policy)
- `connection_rate_limiting_filter` (rate, interval)
- `admin_auth` (compared by token file path, not token values)

At the server level, `threads` and `work_stealing` are also compared because they are set on
Pingora's `ServerConf` at construction time and cannot be changed in a running process.

Changes to any other field (routes, services, devices, DNS interval, observability, TLS
automation, CA file) are classified as runtime-only and handled by the ArcSwap path.

## Error handling

| Failure | Effect |
|---------|--------|
| New config fails validation | Reload aborted, old process undisturbed |
| New process fails to spawn | Error logged, old process continues |
| FD transfer times out | New process exits (bootstrap failure), old process continues |
| New process crashes after FD transfer | Connections on those FDs are lost |
| `pid_file` not configured | Automatic upgrade disabled, error logged |
