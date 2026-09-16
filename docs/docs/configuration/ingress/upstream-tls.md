---
title: Upstream TLS
---

By default, upstream connections use plain HTTP.
To connect to an upstream over TLS, add a `tls` block inside the `endpoint`, or inside the `sock` of a Unix domain socket upstream.
See [Upstreams](upstreams.md) for the parent structure.

```hcl
endpoint = {
  host = "10.0.0.1"
  port = 8443
  tls = {
    # SNI hostname sent during the TLS handshake.
    sni     = "backend.internal"

    # Whether to verify the upstream certificate.
    verify  = true

    # CA certificate for verification.
    ca_file = "/path/to/certs/ca.pem"
  }
}
```

A Unix domain socket upstream uses the same `tls` block inside `sock`.
A socket has no host name, so Snakeway checks the certificate against the `sni` value.

```hcl
sock = {
  path = "/run/app.sock"
  tls = {
    sni     = "app.internal"
    verify  = true
    ca_file = "/path/to/certs/ca.pem"
  }
}
```

## Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `tls.sni` | `string` | (required) | The SNI hostname sent during the TLS handshake. |
| `tls.verify` | `boolean` | (required) | Whether to verify the upstream's certificate. Set to `false` only in controlled environments. |
| `tls.ca_file` | `string` | (optional) | Path to a PEM file of CA certificates used to verify the upstream's certificate in place of the trusted roots of the operating system. Falls back to the global `server.ca_file` if not set. |

## Certificate requirements

Sometimes an upstream that worked with an earlier version of Snakeway fails the TLS handshake after an upgrade.
Earlier versions verified upstream certificates with OpenSSL.
Snakeway now uses rustls, which rejects some certificates that OpenSSL accepts.
When `verify` is `true`, a certificate that breaks one of the rules in this section fails the handshake.
The request to that upstream then fails.
None of these rules apply when `verify` is `false`, because Snakeway does not check the upstream certificate in that case.

### The hostname must be in a subject alternative name

Snakeway compares the `sni` value with the subject alternative name (SAN) entries of the upstream certificate.
A certificate that names the host only in its common name (CN) fails, because the CN is not compared.
An IP address is not accepted as `sni` when `verify` is `true`.

For example, this command prints the SAN entries of a certificate:

```sh
openssl x509 -in upstream.pem -noout -ext subjectAltName
```

### The upstream certificate must not be a CA certificate

Sometimes you trust a self-signed upstream by putting its own certificate in `ca_file`.
rustls accepts this only when the certificate does not have the `CA:TRUE` basic constraint.
The `openssl req -x509` command adds `CA:TRUE` by default.
A self-signed certificate made with the default settings therefore fails the handshake.

To make a self-signed certificate that works, set the basic constraint to `CA:FALSE` and add a SAN entry.
For example, this command makes a certificate and key for `backend.internal`:

```sh
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
  -keyout backend.key -out backend.pem -days 365 \
  -subj "/CN=backend.internal" \
  -addext "basicConstraints=critical,CA:FALSE" \
  -addext "subjectAltName=DNS:backend.internal"
```

Configure the upstream server with `backend.key` and `backend.pem`, and set `ca_file` to `backend.pem`.

If you have several upstreams, you can sign each upstream certificate with a private CA instead.
With a private CA, one `ca_file` that holds the CA certificate verifies every upstream certificate that the CA signs.

### Protocol versions and signature algorithms

Snakeway connects to upstreams with TLS 1.2 or TLS 1.3.
An upstream that supports only TLS 1.0 or TLS 1.1 fails the handshake.

The upstream certificates and keys must use one of these signature algorithms:

- RSA with SHA-256, SHA-384, or SHA-512, using a key from 2048 to 8192 bits
- ECDSA with SHA-256 or SHA-384, using a P-256 or P-384 key
- Ed25519

A certificate signed with SHA-1 fails the handshake.
A P-521 key also fails.

### CA files

A per-upstream `ca_file` and the global `server.ca_file` must contain only PEM-encoded X.509 certificates.
Snakeway checks each certificate when it loads the configuration.
A file that holds a placeholder or a damaged certificate stops the configuration from loading.
A `PRIVATE KEY`, `RSA PRIVATE KEY`, `EC PRIVATE KEY`, `PUBLIC KEY`, `X509 CRL`, or `CERTIFICATE REQUEST` section in the file also stops it.

When a CA file applies to an upstream, Snakeway trusts only the certificates in that file for the connection.
The trusted roots of the operating system are not used for that upstream.
