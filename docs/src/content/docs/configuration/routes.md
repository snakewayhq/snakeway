---
title: Route Configuration
---

Routes define how incoming requests are matched and handled. Each route maps a URL path prefix to either:

- **service** (proxied upstream traffic)
- **static filesystem directory** (local file serving)

Snakeway uses **longest-prefix matching**, meaning more specific routes take precedence over broader ones.

## Modular Configuration

Snakeway supports a modular configuration system. Instead of defining all routes in a single file, you can organize them
into multiple files and include them in the main `snakeway.toml`.

In your `snakeway.toml`, use the `[include]` section to specify where to find route configuration files:

```toml
[include]
routes = "/etc/snakeway/routes.d/*.toml"
```

Each included file can contain one or more `[[service_route]]` and `[[static_route]]` blocks.

## Overview

Routes are defined using one or more `[[service_route]]` and `[[static_route]]` blocks. These blocks can be placed in
the main configuration file or in included files.

## Service Routes

Service routes forward requests to an upstream service.

### Example

```toml
[[service_route]]
path = "/api"
service = "api"
allow_websocket = true
```

### Fields

##### path

**Type:** `string`  
**Required:** `true`

The URL path prefix to match. Must:

- start with `/`
- be unique across all routes

##### service

**Type:** `string`  
**Required:** `true`

The name of the service to proxy requests to. This must match a service defined in your service configuration.

##### allow_websocket

**Type:** `boolean`  
**Default:** `false`

Enables WebSocket upgrades for this route.

##### ws_max_connections

**Type:** `integer`  
**Optional**

Maximum number of concurrent WebSocket connections allowed for this route.

## Static Routes

Static routes serve files from the local filesystem.

### Example

```toml
[[static_route]]
path = "/assets"
file_dir = "/var/www/html"
index = "index.html"
```

### Fields

##### path

**Type:** `string`  
**Required:** `true`

The URL path prefix to match.

##### file_dir

**Type:** `string`  
**Required:** `true`

Absolute path to the directory on disk that will be served.

Constraints:

- must be an absolute path
- must exist
- must be a directory
- must not be `/`

##### index

**Type:** `string`  
**Optional**

Filename to serve when a directory is requested.

##### directory_listing

**Type:** `boolean`  
**Default:** `false`

Whether to enable directory listings when no index file is present.

### Advanced Static Configuration

Static routes include optional configuration for performance and caching.

#### static_config

**Type:** `object`  
**Optional**

Advanced configuration for static file handling.

- `max_file_size`: (integer) Max file size in bytes. Default: `10485760` (10 MiB)
- `small_file_threshold`: (integer) Threshold for small file optimization in bytes. Default: `262144` (256 KiB)
- `min_gzip_size`: (integer) Minimum size to enable gzip compression. Default: `1024` (1 KiB)
- `min_brotli_size`: (integer) Minimum size to enable brotli compression. Default: `4096` (4 KiB)
- `enable_gzip`: (boolean) Enable gzip compression. Default: `true`
- `enable_brotli`: (boolean) Enable brotli compression. Default: `true`

Example:

```toml
[[static_route]]
path = "/static"
file_dir = "/var/www"
static_config = { enable_gzip = false, max_file_size = 1048576 }
```

#### cache_policy

**Type:** `object`  
**Optional**

Configuration for the `Cache-Control` header.

- `max_age`: (integer) `max-age` value in seconds. Default: `3600` (1 hour)
- `public`: (boolean) Whether the cache is `public`. Default: `true`
- `immutable`: (boolean) Whether to add the `immutable` directive. Default: `false`

Example:

```toml
[[static_route]]
path = "/assets"
file_dir = "/var/www/assets"
cache_policy = { max_age = 86400, immutable = true }
```
