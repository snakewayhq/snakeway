# Static Files

Snakeway can serve static files directly from the filesystem, making it easy to host frontend assets, images, and other static content without needing a separate web server.

## Enabling Static Files

Static file serving is an optional feature. To enable it, compile Snakeway with the `static_files` feature:

```bash
cargo build --release --features static_files
```

## Configuration

To serve static files, add a route with `file_dir` instead of `upstream`:

```toml
[[routes]]
path = "/"
file_dir = "./public"
index = true
```

### Configuration Options

| Option | Type | Required | Description |
|--------|------|----------|-------------|
| `path` | string | Yes | The URL path prefix to match |
| `file_dir` | string | Yes | The directory containing static files |
| `index` | boolean | No | Whether to serve `index.html` for directory requests (default: `false`) |

### Example Configurations

**Serve a single-page application:**

```toml
[[routes]]
path = "/"
file_dir = "./dist"
index = true
```

**Serve static assets under a prefix:**

```toml
[[routes]]
path = "/static"
file_dir = "./assets"
index = false
```

**Mix static files with API proxy:**

```toml
# API requests go to upstream
[[routes]]
path = "/api"
upstream = "127.0.0.1:8080"

# Everything else serves static files
[[routes]]
path = "/"
file_dir = "./public"
index = true
```

## Features

### Automatic MIME Type Detection

Snakeway automatically detects and sets the correct `Content-Type` header based on file extensions.

Common types include:

- `.html` → `text/html`
- `.css` → `text/css`
- `.js` → `application/javascript`
- `.json` → `application/json`
- `.png` → `image/png`
- `.jpg`, `.jpeg` → `image/jpeg`
- `.svg` → `image/svg+xml`
- `.wasm` → `application/wasm`

### Caching and Conditional Requests

Snakeway implements HTTP caching headers to reduce bandwidth and improve performance:

- **ETag**: A weak ETag is generated from the file size and modification time
- **Last-Modified**: The file's modification timestamp is sent as an HTTP date

Clients can use conditional requests to avoid re-downloading unchanged files:

- **If-None-Match**: If the client's cached ETag matches, Snakeway returns `304 Not Modified`
- **If-Modified-Since**: If the file hasn't changed since the given date, Snakeway returns `304 Not Modified`

### Compression

Snakeway automatically compresses responses for clients that support it, reducing transfer sizes significantly for text-based content.

**Supported encodings (in order of preference):**

1. **Brotli** (`br`) - Best compression ratio, preferred when client supports it
2. **gzip** - Fallback for clients that don't support Brotli

**Compression behavior:**

- Only compressible MIME types are compressed (text, JSON, JavaScript, XML, SVG, WASM, etc.)
- Brotli is used for files ≥ 4 KiB
- gzip is used for files ≥ 256 bytes (and < 4 KiB when Brotli would be preferred)
- Compression is skipped if the compressed size isn't smaller than the original
- The `Vary: Accept-Encoding` header is added for proper cache behavior

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

### File Streaming

- **Small files** (≤ 256 KiB): Read entirely into memory, compressed if applicable
- **Large files** (> 256 KiB): Streamed directly from disk in 32 KiB chunks (compression not applied to preserve streaming)

### Security

Snakeway includes several security measures to protect against common attacks:

- **Path traversal protection**: Requests containing `..` or attempting to escape the `file_dir` are rejected with `403 Forbidden`
- **File size limit**: Files larger than 10 MiB are rejected to prevent memory exhaustion
- **Symlink resolution**: Paths are canonicalized to prevent symlink-based escapes

## Limitations

- Compression is only applied to small files (≤ 256 KiB) to maintain streaming capability for large files
- Directory listing is not supported; only explicit file requests are served
- Range requests (partial content) are not currently supported
