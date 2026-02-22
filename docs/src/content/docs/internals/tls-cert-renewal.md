---
title: TLS Certificate Renewal
---

The following happens when a certificate is "renewed" with Let's Encrypt:

1. Generate a new private key (usually)
2. Create a new ACME order
3. Prove domain control (challenge)
4. Finalize the order with a CSR
5. Download the new certificate

