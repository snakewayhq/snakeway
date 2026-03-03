---
title: TLS Certificate Renewal
---

The following happens when a certificate is issued with Let's Encrypt:

1. Generate a new private key (usually)
2. Create a new ACME order
3. Prove domain control (challenge)
4. Finalize the order with a CSR
5. Download the new certificate

The `CertManager` is responsible for this.

An implementation of the `CertStore` trait determines where certificates are stored.

### TLS Automation

When `tls_automation` is configured in `snakeway.hcl`, Snakeway starts a background `CertManager` task.

On startup (and periodically thereafter), the `CertManager` checks each ACME-managed domain. If a certificate is
missing or due to expire within `renew_within_days`, it initiates the ACME renewal flow described above.

The renewed certificate is stored and immediately made available to new TLS handshakes without requiring a server
restart or reload.

### SNI

The SNI handling is a bit tricky.

Pingora has a mechanism, via its `TlsAccept` trait, for hooking into the TLS handshake.

Snakeway uses this feature to achieve two things:

1. Load ACME-generated certs based on **SNI**.
2. Make the **SNI** available to the request pipeline via Pingora's `SslDigest`.

```rust
impl TlsAccept for SnakewayTlsAccept {
    async fn certificate_callback(&self, ssl: &mut TlsRef) {
        match &self.cert_mode {
            CertMode::Manual => {
                // Do nothing. Cert already configured in settings file.
            }
            CertMode::Acme(state) => {
                // Perform dynamic lookup and install cert based on SNI
                acme_lookup_and_set_cert(state, ssl).await
            }
        }
    }

    async fn handshake_complete_callback(
        &self,
        ssl: &TlsRef,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        // Extract SNI.
        let hostname = extract_sni(ssl).filter(|s| !s.is_empty())?;
        let hostname = Arc::new(DownstreamSni(hostname.clone()));
        Some(hostname)
    }
}
```

The tricky part is extracting the SNI in the `HttpProxy` implementation.

```rust
let maybe_sni = & ssl_digest.extension.get::<DownstreamSni>();
```

:::note
This is an **important note** because this data crosses the boundary from L4 to L7 automagically via Pingora.
:::
