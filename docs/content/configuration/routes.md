# Route Configuration

Routes define how incoming requests are matched and handled. Each route maps a URL path prefix to either:

- a **service** (proxied upstream traffic), or
- a **static filesystem directory** (local file serving).

Snakeway uses **longest-prefix matching**, meaning more specific routes take precedence over broader ones.

---

## Overview

Routes are defined using one or more `[[route]]` blocks.

```toml
[[route]]
type = "service"
path = "/api"
service = "api"

[[route]]
type = "static"
path = "/static"
dir  = "/var/www"
```

Each route must explicitly declare its `type` to avoid ambiguity and enable strict validation.

---

## Route Types

### type = "service"

A **service route** forwards traffic to a named service defined in the service configuration.

#### Example

```toml
[[route]]
type = "service"
path = "/api"
service = "api"
```

#### Fields

##### type

**Type:** `string`  
**Required:** `true`  
**Allowed values:** `service`, `static`

Declares this route as a service-backed route.

---

##### path

**Type:** `string`  
**Required:** `true`

The URL path prefix to match. Must:

- start with `/`
- be unique across all routes

Examples:
- `/api`
- `/ws`
- `/`

---

##### service

**Type:** `string`  
**Required:** `true`

The name of the service to route requests to.  
Must match a service defined in the service configuration.

---

##### allow_websocket

**Type:** `boolean`  
**Default:** `false`

Enables WebSocket upgrades for this route.

Only valid for `type = "service"` routes.

---

##### ws_idle_timeout_ms

**Type:** `integer`  
**Optional**

Idle timeout for WebSocket connections, in milliseconds.

Only applicable when `allow_websocket = true`.

---

##### ws_max_connections

**Type:** `integer`  
**Optional**

Maximum number of concurrent WebSocket connections allowed for this route.

---

### type = "static"

A **static route** serves files directly from the local filesystem.

#### Example

```toml
[[route]]
type = "static"
path = "/static"
dir  = "/var/www"
```

#### Fields

##### type

**Type:** `string`  
**Required:** `true`  
**Value:** `static`

Declares this route as a static file route.

---

##### path

**Type:** `string`  
**Required:** `true`

The URL path prefix to match.

---

##### dir

**Type:** `string`  
**Required:** `true`

Absolute path to the directory on disk that will be served.

Constraints:

- must be an absolute path
- must exist
- must be a directory
- must not be `/`

---

##### index

**Type:** `string`  
**Optional**

Filename to serve when a directory is requested.

Example:

```toml
index = "index.html"
```

---

##### directory_listing

**Type:** `boolean`  
**Default:** `false`

Whether to enable directory listings when no index file is present.

---

## Static File Configuration

Static routes include a file handling configuration block with sane defaults.

These settings are currently not user-configurable per-route and use the defaults shown below.

### Defaults

```text
max_file_size        = 10 MiB
small_file_threshold = 256 KiB
min_gzip_size        = 1 KiB
min_brotli_size      = 4 KiB
enable_gzip          = true
enable_brotli        = true
```

---

## Static Cache Policy

Static routes also include a cache policy with defaults optimized for typical web assets.

### Defaults

```text
max_age   = 3600 seconds
public    = true
immutable = false
```

---

## Validation Rules

Snakeway enforces strict validation at startup:

- Route paths must be unique and absolute
- `type` must be explicitly declared
- `type = "service"` routes:
  - must define `service`
  - must not define `dir`
- `type = "static"` routes:
  - must define `dir`
  - must not define `service`
  - must not enable WebSockets
- Static directories must exist and be safe to serve

Invalid configurations will fail fast with clear diagnostics.

---

## Notes

- Route matching is prefix-based, not regex-based
- WebSocket support is opt-in per route
- gRPC traffic should use `type = "service"` routes with HTTP/2 enabled at the listener level
