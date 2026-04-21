---
title: TLS Automation
---

The `tls_automation` block configures automatic certificate issuance and renewal via the ACME protocol (for example, Let's Encrypt). When present, Snakeway will automatically obtain and renew certificates for any `bind` blocks configured with `mode = "acme"`.

See the [TLS Cert Management](/docs/guide/tls-cert-management) guide for a full walkthrough.

## Configuration Example

```hcl
tls_automation = {
  renew_within_days = 30                   # Days before expiry to attempt renewal

  acme = {
    directory_url = "https://acme-v02.api.letsencrypt.org/directory"  # ACME directory URL
    data_dir      = "/var/lib/snakeway/acme"                          # Persistent state directory
    contact_email = ["admin@example.com"]                             # Registration emails
  }

  cert_store = {
    type     = "filesystem"                           # Storage backend type
    cert_dir = "/var/lib/snakeway/acme/certs"         # On-disk certificate directory
  }
}
```

## Field Reference

`renew_within_days` integer, default: `30`. How many days before certificate expiry Snakeway begins renewal attempts. Recommended range: 7 to 30.

`acme` object, **required**. ACME server connection details. See [ACME Configuration](acme.md).

`cert_store` object, **required**. Where issued certificates are stored. See [Certificate Store](cert-store.md).
