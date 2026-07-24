---
title: Static Files
---

An ingress configuration file may define zero or more static file policies. Static file policies serve files directly
from the local filesystem without proxying to an upstream.

```hcl
static_files = [
  {
    routes = [
      {
        # Match requests for these hostnames.
        hosts = ["example.com"]

        # Match requests with this path prefix.
        path = "/assets"

        # Absolute path to the directory on disk.
        file_dir = "/var/www/html"

        # Filename to serve when a directory is requested.
        index = "index.html"

        # Whether to show directory listings.
        directory_listing = false

        # Maximum file size in bytes (10 MiB).
        max_file_size = 10485760

        # See: Compression, Cache Policy
        compression = { ... }
        cache_policy = { ... }
      }
    ]
  }
]
```

## Fields

| Field               | Type           | Default    | Description                                                                                                                                                                                                                   |
|---------------------|----------------|------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `hosts`             | `list(string)` | (required) | The list of hostnames this static route applies to. Requests are only matched if the `Host` header matches one of the specified values. Use `["*"]` to match all hostnames.                                                   |
| `path`              | `string`       | (required) | The URL path prefix to match.                                                                                                                                                                                                 |
| `file_dir`          | `string`       | (required) | Absolute path to the directory on disk that will be served. Must exist, must be a directory, and must not be `/`.                                                                                                             |
| `index`             | `string`       | (optional) | Filename to serve when a directory is requested.                                                                                                                                                                              |
| `directory_listing` | `boolean`      | (required) | Whether to enable directory listings when no index file is present.                                                                                                                                                           |
| `max_file_size`     | `integer`      | (required) | Maximum file size in bytes that will be served. A file larger than this is rejected with `403 Forbidden`, so the value must be at least 1 and at most 68719476736 (64 GiB). 10485760 (10 MiB) is a reasonable starting point. |
| `compression`       | `object`       | (required) | Compression settings. See [Compression](compression.md).                                                                                                                                                                      |
| `cache_policy`      | `object`       | (required) | Cache-Control header configuration. See [Cache Policy](cache-policy.md).                                                                                                                                                      |
