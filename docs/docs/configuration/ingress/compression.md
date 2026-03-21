---
title: Compression
---

The `compression` block configures response compression for [static file](static-files.md) routes.

```hcl
compression = {
  # Enable gzip compression.
  enable_gzip          = true

  # Enable brotli compression.
  enable_brotli        = true

  # Threshold for small file optimization, in bytes.
  small_file_threshold = 262144

  # Minimum file size for gzip, in bytes.
  min_gzip_size        = 1024

  # Minimum file size for brotli, in bytes.
  min_brotli_size      = 4096
}
```

## Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `enable_gzip` | `boolean` | `true` | Enable gzip compression. |
| `enable_brotli` | `boolean` | `true` | Enable brotli compression. |
| `small_file_threshold` | `integer` | `262144` | Threshold for small file optimization, in bytes. Defaults to 256 KiB. |
| `min_gzip_size` | `integer` | `1024` | Minimum file size to enable gzip compression, in bytes. Defaults to 1 KiB. |
| `min_brotli_size` | `integer` | `4096` | Minimum file size to enable brotli compression, in bytes. Defaults to 4 KiB. |
