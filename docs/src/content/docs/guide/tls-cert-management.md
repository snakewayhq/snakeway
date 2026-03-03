---
title: TLS Cert Management
---

Snakeway supports certificate renewal with Let's Encrypt (via the ACME protocol).

This requires specific configuration, but should otherwise be automatic.

Snakeway also supports manual certificate management.

## Let's Encrypt (ACME protocol)

Out-of-the-box, `filesystem` and `memory` stores are supported.
You would use `filesystem` in practice.
The `memory` store is for development and testing.
In the future, other stores (e.g., S3, Consul) will be supported.

The `renew_within_days` param specifies when renewal attempts begin.
This should be from 7 to 30 days before the certificate expires.

The `filesystem` store configuration:

```hcl
server {
  // ...
  tls_automation = {
    renew_within_days = 30
    acme = {
      directory_url = "https://acme-v02.api.letsencrypt.org/directory"
      data_dir      = "/var/lib/snakeway/acme"
      contact_email = ["admin@example.com"]
    }
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
  tls_automation = {
    renew_within_days = 30
    acme = {
      directory_url = "https://acme-v02.api.letsencrypt.org/directory"
      data_dir      = "/var/lib/snakeway/acme"
      contact_email = ["admin@example.com"]
    }
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
    mode      = "acme"
    domains   = ["example.com", "api.example.com"]
    challenge = "http01"
  }
}
```

## Manual Certificate Management

For manual certificate management, an ingress file should have a bind block that specifies the manually created files:

```hcl
bind = {
  // ...
  tls = {
    mode = "manual"
    cert = "/path/to/certs/server.pem"
    key  = "/path/to/certs/server.key"
  }
}
```
