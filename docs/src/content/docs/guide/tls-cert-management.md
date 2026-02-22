---
title: TLS Cert Management
---

Snakeway supports certificate renewal with Let's Encrypt (via the ACME protocol).

This requires specific configuration, but should otherwise be automatic.

Snakeway also supports manual certificate management.

## Let's Encrypt (ACME protocol)

Out-of-the-box, `filesystem` and `memory` stores are supported.
You would use `filesystem` in practive.
The `memory` store is for development and testing.
In the future, other stores (e.g., S3, Consul) will be supported.

The `renew_within_days` param specifies when renewal attempts begin.
This should be from 7 to 30 days before the certificate expires.

The `filesystem` store configuration:

```hcl
server {
  // ...
  tls = {
    renew_within_days = 30
    cert_store = {
      type     = "filesystem"
      cert_dir = "/var/lib/snakeway/acme/certs"
    }
  }
}
```

The `memory` store configuration:

```hcl
server {
  // ...
  tls = {
    renew_within_days = 30
    cert_store = {
      type = "memory"
    }
  }
}
```

Your ingress files must be configured appropriately.

```hcl
bind = {
  // ...
  tls = {
    mode = "acme" // <- "acme" for automatic cert renewal.
    domains = ["example.com", "api.example.com"]
    challenge = "http-01"
  }
}
```

## Manual Certificate Management

For manual certificate management, an ingress file should have a bind block that specifies the manually created files:

```hcl
bind = {
  // ...
  tls = {
    mode = "static" // <- "static" for local file certs.
    cert = "/path/to/certs/server.pem"
    key  = "/path/to/certs/server.key"
  }
}
```
