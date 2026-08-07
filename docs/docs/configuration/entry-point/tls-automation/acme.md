---
title: ACME Configuration
---

The `acme` block inside `tls_automation` configures the connection to an ACME certificate authority.

## Configuration Example

```hcl
acme = {
  directory_url = "https://acme-v02.api.letsencrypt.org/directory"  # ACME directory URL
  data_dir = "/var/lib/snakeway/acme"                          # Persistent state directory
  contact_email = ["admin@example.com"]                             # Registration emails
  ca_file = "/path/to/acme-ca.pem"                           # Optional; CA for ACME server TLS
}
```

## Field Reference

`directory_url` string, **required**.
The ACME directory URL.
For production use with Let's Encrypt, set this to `https://acme-v02.api.letsencrypt.org/directory`.
For staging, use the Let's Encrypt staging URL.

`data_dir` string, **required**.
Directory for persisting ACME account keys and order state.
This directory must exist before Snakeway starts.
It will not be created automatically.
Snakeway must have read and write access to this path.

`contact_email` list of strings, **required**.
One or more email addresses registered with the ACME provider.
The certificate authority uses these addresses for expiry warnings and account recovery.

`ca_file` string, default: none.
CA certificate file for verifying the ACME server's TLS certificate.
This is useful for staging or testing environments that use a private CA.
