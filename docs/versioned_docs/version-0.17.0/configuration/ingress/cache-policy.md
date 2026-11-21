---
title: Cache Policy
---

The `cache_policy` block configures the `Cache-Control` response header for [static file](static-files.md) routes.

```hcl
cache_policy = {
  # The max-age value in seconds.
  max_age_seconds = 3600

  # Whether the cache is public.
  public          = true

  # Whether to add the immutable directive.
  immutable       = false
}
```

## Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `max_age_seconds` | `integer` | `3600` | The `max-age` value in seconds. Defaults to 1 hour. |
| `public` | `boolean` | `true` | Whether the cache is `public`. |
| `immutable` | `boolean` | `false` | Whether to add the `immutable` directive. |
