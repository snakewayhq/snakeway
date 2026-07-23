---
title: Static Files
---


Snakeway can serve static files directly from the filesystem, making it easy to host frontend assets, images, and other
static content without needing a separate web server.

## Configuration

To serve static files, add a route to the `static_files` block:

```hcl
static_files = [
  {
    routes = [
      {
        hosts             = ["*"]
        path              = "/"
        file_dir          = "/var/www/public"
        index             = "index.html"
        directory_listing = false
        max_file_size     = 10485760
        compression       = {}
        cache_policy      = {}
      }
    ]
  }
]
```

Every field above except `index` is required.
Write `compression = {}` and `cache_policy = {}` to accept the defaults for those blocks, or set the individual
options described below.

### Configuration Options

| Option              | Type    | Required | Description                                                 |
|---------------------|---------|----------|-------------------------------------------------------------|
| `path`              | string  | Yes      | The URL path prefix to match                                |
| `file_dir`          | string  | Yes      | The directory containing static files                       |
| `hosts`             | list    | Yes      | The hostnames this route applies to                         |
| `index`             | string  | No       | The name of the index file, e.g., `index.html` (no default) |
| `directory_listing` | boolean | Yes      | Whether list the contents of directory requests             |
| `max_file_size`     | integer | Yes      | Largest file that will be served, in bytes (see below)      |
| `cache_policy`      | object  | Yes      | Advanced cache policy configuration (see below)             |
| `compression`       | object  | Yes      | Advanced compression configuration (see below)              |

### Cache Policy (Per-Route)

Each static route carries a `cache_policy` block. Every option within it has a default, so `cache_policy = {}` is
valid.

| Option            | Type    | Default | Description                                                                                                                  |
|-------------------|---------|---------|------------------------------------------------------------------------------------------------------------------------------|
| `max_age_seconds` | integer | `3600`  | How long a cached response is valid (e.g., `3600 seconds` = `1 hour`).                                                       |
| `public`          | boolean | `true`  | Indicates a cache can be shared across domains or with third-party services.                                                 |
| `immutable`       | boolean | `false` | Response won't change unless its associated resource changes, allowing caches to return the same result without re-checking. |

### Advanced Compression (Per-Route)

Each static route carries a `compression` block. Every option within it has a default, so `compression = {}` is valid.

| Option                 | Type    | Default  | Description                                                                                                 |
|------------------------|---------|----------|-------------------------------------------------------------------------------------------------------------|
| `enable_brotli`        | boolean | `true`   | Enable Brotli compression for compressible content                                                          |
| `enable_gzip`          | boolean | `true`   | Enable gzip compression (fallback when Brotli is unavailable)                                               |
| `min_brotli_size`      | integer | `4096`   | Minimum file size in bytes to apply Brotli compression (4 KiB)                                              |
| `min_gzip_size`        | integer | `1024`   | Minimum file size in bytes to apply gzip compression (1 KiB)                                                |
| `small_file_threshold` | integer | `262144` | Files smaller than this (in bytes) are read into memory and compressed; larger files are streamed (256 KiB) |

**Example with custom compression settings:**

```hcl
static_files = [
  {
    routes = [
      {
        hosts             = ["*"]
        path              = "/"
        file_dir          = "/var/www/public"
        directory_listing = false
        max_file_size     = 10485760
        cache_policy      = {}
        compression = {
          enable_brotli   = true
          enable_gzip     = true
          min_brotli_size = 4096
          min_gzip_size   = 1024
        }
      }
    ]
  }
]
```

**Disable compression entirely for a route:**

```hcl
static_files = [
  {
    routes = [
      {
        hosts             = ["*"]
        path              = "/raw"
        file_dir          = "/var/www/raw-assets"
        directory_listing = false
        max_file_size     = 10485760
        cache_policy      = {}
        compression = {
          enable_brotli = false
          enable_gzip   = false
        }
      }
    ]
  }
]
```

**Serve large assets:**

`max_file_size` is a route field rather than a compression option.
It caps the size of any file the route will serve, and a larger file is rejected with `403 Forbidden` before it is
opened, including a range request for a small slice of it.
The accepted range is 1 to 68719476736 (64 GiB).

```hcl
static_files = [
  {
    routes = [
      {
        hosts             = ["*"]
        path              = "/downloads"
        file_dir          = "/var/www/large-files"
        directory_listing = false
        max_file_size     = 104857600 # 100 MiB
        cache_policy      = {}
        compression = {
          small_file_threshold = 1048576 # 1 MiB, stream files larger than this
        }
      }
    ]
  }
]
```

### Example Configurations

**Serve a single-page application:**

```hcl
static_files = [
  {
    routes = [
      {
        hosts             = ["*"]
        path              = "/"
        file_dir          = "/var/www/dist"
        index             = "index.html"
        directory_listing = false
        max_file_size     = 10485760
        compression       = {}
        cache_policy      = {}
      }
    ]
  }
]
```

**Serve static assets under a prefix:**

```hcl
static_files = [
  {
    routes = [
      {
        hosts             = ["*"]
        path              = "/static"
        file_dir          = "/var/www/assets"
        directory_listing = false
        max_file_size     = 10485760
        compression       = {}
        cache_policy      = {}
      }
    ]
  }
]
```

**Mix static files with API proxy:**

```hcl
services = [
  {
    routes = [
      {
        hosts = ["*"]
        path  = "/api"
      }
    ]
    upstreams = [
      {
        endpoint = { host = "127.0.0.1", port = 8080 }
      }
    ]
  }
]

static_files = [
  {
    routes = [
      {
        hosts             = ["*"]
        path              = "/"
        file_dir          = "/var/www/public"
        directory_listing = false
        max_file_size     = 10485760
        compression       = {}
        cache_policy      = {}
      }
    ]
  }
]
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

:::note[Large Files]
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

See [Advanced Compression (Per-Route)](#advanced-compression-per-route) for customization options.

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

