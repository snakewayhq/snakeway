---
title: Network Policy Device
---

The **Network Policy device** is a builtin Snakeway device that enforces **coarse‑grained network trust boundaries**
based on resolved client identity.

It operates after the **Identity device** and makes **binary allow / deny decisions** using authoritative identity facts
rather than raw headers or connection metadata.

This device is intentionally **simple**, **fail‑closed**, and designed to express clear operator intent.

## Design Goals

The Network Policy device is designed around a small set of strict principles:

* Enforce **clear network trust boundaries**
* Operate on **authoritative identity**, not untrusted request headers
* Fail **early and deterministically**
* Avoid per‑request state or complex policy logic
* Remain easy to reason about during audits and incident response

It answers the question:
> *Should this client be allowed to access this service at all?*

## What Gets Enforced

Depending on configuration, the Network Policy device can enforce:

* Client IP allowlists (CIDR‑based)
* Whether forwarded requests are allowed
* How invalid forwarded identity claims are handled

All enforcement occurs during **`on_request`**, before the request is proxied upstream.

## Identity Dependency

The Network Policy device consumes the following fields from `ClientIdentity`:

* `ip` — the resolved client IP address
* `is_forwarded` — whether the request contained forwarded identity
* `is_trusted` — whether forwarded identity claims are trusted

These fields are produced **once** by the Identity device and are treated as **authoritative and immutable** for the
lifetime of the request.

If Identity is not present, the Network Policy device does nothing.

## CIDR Allow List

You can restrict access to a set of allowed client networks using CIDR notation.

```hcl
cidr_allow = [
  "10.0.0.0/8",
  "192.168.0.0/16"
]
```

CIDR rules:

* Each entry must be a valid IPv4 or IPv6 CIDR
* If `cidr_allow` is **empty**, all client IPs are allowed
* If `cidr_allow` is **non‑empty**, the client IP **must** match at least one CIDR
* Matching is performed against the **resolved client IP**, not the TCP peer

Requests from disallowed IPs are rejected immediately with `403 Forbidden`.

---

## Forwarded Request Handling

Forwarded requests are those that include forwarded identity (for example via `X‑Forwarded‑For`) and are resolved by the
`Identity` device.

The Network Policy device allows operators to control whether such requests are permitted.

### Allow or Deny Forwarded Requests

```hcl
forwarding {
  allow = true
}
```

Rules:

* If `allow = false`, **all forwarded requests are denied**
* Direct (non‑forwarded) requests are unaffected

This is useful for services that must only be accessed directly, without proxies.

## Invalid Forwarded Identity

When forwarded requests are allowed, you can control how invalid forwarded identity claims are handled.

```hcl
forwarding {
  allow      = true
  on_invalid = "deny"
}
```

Options:

* **`deny`** — reject requests with untrusted forwarded identity
* **`ignore`** — allow the request to proceed using the resolved client IP

An invalid forwarded identity may occur when:

* Forwarded headers are present
* The immediate peer is not trusted
* Forwarded headers are malformed or exceed size limits

This setting allows operators to choose between **strict enforcement** and **best‑effort compatibility**.

## Evaluation Order

The Network Policy device evaluates rules in the following order:

1. **CIDR allow list** — deny if client IP is not allowed
2. **Forwarded request check** — deny if forwarding is disallowed
3. **Forwarded validity check** — deny or ignore invalid forwarded identity

If any step denies the request, processing stops immediately.

## Configuration Example

```hcl
network_policy_device {
  enable = true

  cidr_allow = [
    "10.0.0.0/8",
    "192.168.0.0/16"
  ]

  forwarding {
    allow      = true
    on_invalid = "deny"
  }
}
```

This configuration:

* Allows only private RFC1918 networks
* Permits forwarded requests
* Rejects requests with invalid forwarded identity

