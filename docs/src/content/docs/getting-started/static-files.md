---
title: Static Files
---


Snakeway can serve static files directly from the filesystem, making it easy to host frontend assets, images, and other
static content without needing a separate web server.

## Enabling Static Files

Static file serving is an optional feature. To enable it, compile Snakeway with the `static_files` feature:

```bash
cargo build --release --features static_files
```

## Configuration

To serve static files, add a route with `file_dir` instead of `upstream`:

```toml
[[static_route]]
path = "/"
file_dir = "/var/www/public"
index = "index.html"
```

### Configuration Options

| Option              | Type    | Required | Description                                                        |
|---------------------|---------|----------|--------------------------------------------------------------------|
| `path`              | string  | Yes      | The URL path prefix to match                                       |
| `file_dir`          | string  | Yes      | The directory containing static files                              |
| `index`             | string  | No       | The name of the index file, e.g., `index.html` (no default)        |
| `directory_listing` | boolean | No       | Whether list the contents of directory requests (default: `false`) |
| `cache_policy`      | table   | No       | Advanced cache policy configuration (see below)                    |
| `config`            | table   | No       | Advanced static file configuration (see below)                     |

### Cache Policy (Per-Route)

Each static route can have an optional `[routes.cache_policy]` cache header behavior.

| Option      | Type    | Default | Description                                                                                                                 |
|-------------|---------|---------|-----------------------------------------------------------------------------------------------------------------------------|
| `max_age`   | integer | `3600`  | How long a cached response is valid (e.g., `3600 seconds` = `1 hour`).                                                      |
| `public`    | boolean | `true`  | Indicates a cache can be shared across domains or with third-party services.                                                |
| `immutable` | boolean | `false` | esponse won't change unless its associated resource changes, allowing caches to return the same result without re-checking. |

### Advanced Configuration (Per-Route)

Each static route can have an optional `[routes.config]` section to customize compression and file handling behavior.

| Option                 | Type    | Default    | Description                                                                                                 |
|------------------------|---------|------------|-------------------------------------------------------------------------------------------------------------|
| `enable_brotli`        | boolean | `true`     | Enable Brotli compression for compressible content                                                          |
| `enable_gzip`          | boolean | `true`     | Enable gzip compression (fallback when Brotli is unavailable)                                               |
| `min_brotli_size`      | integer | `4096`     | Minimum file size in bytes to apply Brotli compression (4 KiB)                                              |
| `min_gzip_size`        | integer | `1024`     | Minimum file size in bytes to apply gzip compression (1 KiB)                                                |
| `small_file_threshold` | integer | `262144`   | Files smaller than this (in bytes) are read into memory and compressed; larger files are streamed (256 KiB) |
| `max_file_size`        | integer | `10485760` | Maximum file size in bytes that will be served (10 MiB)                                                     |

**Example with custom compression settings:**

```toml
[[static_route]]
path = "/"
file_dir = "/var/www/public"

[routes.config]
enable_brotli = true
enable_gzip = true
min_brotli_size = 4096
min_gzip_size = 1024
```

**Disable compression entirely for a route:**

```toml
[[static_route]]
path = "/raw"
file_dir = "/var/www/raw-assets"

[routes.config]
enable_brotli = false
enable_gzip = false
```

**Increase file size limits for large assets:**

```toml
[[static_route]]
path = "/downloads"
file_dir = "/var/www/large-files"

[routes.config]
max_file_size = 104857600  # 100 MiB
small_file_threshold = 1048576  # 1 MiB - stream files larger than this
```

### Example Configurations

**Serve a single-page application:**

```toml
[[static_route]]
path = "/"
file_dir = "/var/www/dist"
```

**Serve static assets under a prefix:**

```toml
[[static_route]]
path = "/static"
file_dir = "/var/www/assets"
```

**Mix static files with API proxy:**

```toml

[[static_route]]
path = "/api"
service = "127.0.0.1:8080"


[[static_route]]
path = "/"
file_dir = "/var/www/public"
```

## MIME Type Detection

Snakeway automatically detects and sets the correct `Content-Type` header based on file extensions.

Common types include:

| Extension       | MIME Type                |
|-----------------|--------------------------|
| `.html`         | `text/html`              |
| `.css`          | `text/css`               |
| `.js`           | `application/javascript` |
| `.json`         | `application/json`       |
| `.png`          | `image/png`              |
| `.jpg`, `.jpeg` | `image/jpeg`             |
| `.svg`          | `image/svg+xml`          |
| `.wasm`         | `application/wasm`       |

## Caching and Conditional Requests

Snakeway implements HTTP caching headers to reduce bandwidth and improve performance:

- **ETag**: A weak ETag is generated from the file size and modification time
- **Last-Modified**: The file's modification timestamp is sent as an HTTP date

Clients can use conditional requests to avoid re-downloading unchanged files:

- **If-None-Match**: If the client's cached ETag matches, Snakeway returns `304 Not Modified`
- **If-Modified-Since**: If the file hasn't changed since the given date, Snakeway returns `304 Not Modified`

## Compression

Snakeway automatically compresses responses for clients that support it, reducing transfer sizes significantly for
text-based content.

::: info Large Files
Snakeway supports compression for static files when the response size is known in advance.
Large static files are streamed without compression to preserve range requests and content length semantics.
Support for precompressed assets is planned.
:::

**Supported encodings (in order of preference):**

1. **Brotli** (`br`) - Best compression ratio, preferred when client supports it
2. **gzip** - Fallback for clients that don't support Brotli

**Compression behavior (default settings):**

- Only compressible MIME types are compressed (text, JSON, JavaScript, XML, SVG, WASM, etc.)
- Brotli is used for files `≥ 4 KiB` (configurable via `min_brotli_size`)
- gzip is used for files `≥ 1 KiB` when Brotli is unavailable or not preferred by the client (configurable via
  `min_gzip_size`)
- Compression can be disabled per-route using `enable_brotli` and `enable_gzip` options
- Compression is skipped if the compressed size isn't smaller than the original
- The `Vary: Accept-Encoding` header is added for proper cache behavior

See [Advanced Configuration (Per-Route)](#advanced-configuration-per-route) for customization options.

**Example request/response:**

```http
GET /app.js HTTP/1.1
Accept-Encoding: gzip, deflate, br
```

```http
HTTP/1.1 200 OK
Content-Type: application/javascript
Content-Encoding: br
Vary: Accept-Encoding
ETag: W/"1a2b3c-4d5e6f"
```

## File Streaming

- **Small files** (≤ 256 KiB by default): Read entirely into memory, compressed if applicable
- **Large files** (> 256 KiB by default): Streamed directly from disk in 32 KiB chunks. Streaming responses are not
  compressed, since compression currently requires buffering the entire file in memory.

The threshold can be adjusted per-route using the `small_file_threshold` option.

## Security

Snakeway includes several security measures to protect against common attacks:

- **Path traversal protection**: Requests containing `..` or attempting to escape the `file_dir` are rejected with
  `403 Forbidden`
- **File size limit**: Files larger than 10 MiB (by default) are rejected to prevent memory exhaustion (configurable
  per-route).
- **Symlink resolution**: Paths are canonicalized to prevent symlink-based escapes

