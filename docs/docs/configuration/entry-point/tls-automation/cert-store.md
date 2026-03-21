---
title: Certificate Store
---

The `cert_store` block inside `tls_automation` controls where Snakeway stores issued certificates. Two backend types are supported: `filesystem` and `memory`.

## Filesystem Store

The `filesystem` backend persists certificates to disk. Certificates survive process restarts and are available immediately on startup. This is the recommended backend for production deployments.

```hcl
cert_store = {
  type     = "filesystem"                       # Persist to disk
  cert_dir = "/var/lib/snakeway/acme/certs"     # Directory for certificate files
}
```

## Memory Store

The `memory` backend stores certificates in memory only. All certificates are lost when the process exits, and Snakeway must re-issue them on the next startup. This backend is useful for development and testing where persistence is not needed.

```hcl
cert_store = {
  type = "memory"    # In-memory only; lost on restart
}
```

## Field Reference

`type` string, **required**. Storage backend: `"filesystem"` or `"memory"`.

`cert_dir` string, **required** when `type = "filesystem"`. Directory where certificates are written to disk. Snakeway must have read and write access to this path. Ignored when `type = "memory"`.

## Choosing a Backend

For production, use `filesystem`. It avoids unnecessary ACME requests after restarts and ensures certificates are immediately available. The `memory` backend is convenient for local development or CI pipelines where certificate persistence is not a concern.
