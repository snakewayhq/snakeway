---
title: Network Policy Device
---

The **Network Policy device** enforces coarse-grained network trust boundaries based on the resolved client identity.
It requires the [Identity device](/docs/configuration/devices/identity) to be enabled, because it reads the `ClientIdentity` extension that Identity produces rather than parsing headers directly.
The Network Policy device runs after Identity in the device pipeline and makes binary allow/deny decisions during the `on_request` phase, before any upstream connection is established.

## Configuration Example

This configuration allows only private RFC 1918 networks, permits forwarded requests, and rejects requests with invalid forwarded identity claims.

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

## Field Reference

| Field                   | Type            | Default    | Description                                                                                                                                        |
|-------------------------|-----------------|------------|----------------------------------------------------------------------------------------------------------------------------------------------------|
| `enable`                | bool            | `false`    | Enables the Network Policy device.                                                                                                                 |
| `cidr_allow`            | list of strings | `[]`       | CIDR ranges from which clients are permitted. An empty list causes a configuration error.                                                          |
| `forwarding.allow`      | bool            | `false`    | Whether requests arriving via a forwarded header (such as `X-Forwarded-For`) are permitted.                                                        |
| `forwarding.on_invalid` | string          | `"ignore"` | How to handle forwarded requests with untrusted or malformed identity. `"deny"` rejects them. `"ignore"` allows them using the resolved client IP. |
| `paths`                 | list of strings | `[]`       | Path prefixes this device applies to. Empty means all paths. See [Path Scoping](#path-scoping).                                                    |

## CIDR Allow List

The `cidr_allow` field restricts access to a set of permitted client networks specified in CIDR notation.
Each entry must be a valid IPv4 or IPv6 CIDR block.

```hcl
cidr_allow = [
  "10.0.0.0/8",
  "192.168.0.0/16",
  "2001:db8::/32"
]
```

Matching is performed against the **resolved client IP** from the Identity device, not the raw TCP peer address.
This means that if Snakeway is behind a load balancer and the Identity device has correctly resolved the true client IP via trusted proxies, the Network Policy device evaluates the true client rather than the load balancer.

Requests from disallowed IPs are rejected immediately with `403 Forbidden`.

## Forwarded Request Handling

Forwarded requests are those that include a forwarded identity header (such as `X-Forwarded-For`) as resolved by the Identity device.
The Network Policy device lets operators control whether these requests are permitted at all.

```hcl
forwarding {
  allow = true
}
```

When `allow` is set to `false`, all forwarded requests are denied regardless of CIDR rules.
Direct (non-forwarded) requests are unaffected.
This is useful for backend services that must only be accessed directly, without any intermediate proxies.

### Invalid Forwarded Identity

When forwarded requests are allowed, the `on_invalid` field controls what happens when the forwarded identity cannot be verified.
An invalid forwarded identity can occur when the immediate peer is not in the trusted proxies list, when forwarded headers are malformed, or when headers exceed the configured size limits.

```hcl
forwarding {
  allow      = true
  on_invalid = "deny"
}
```

- **`"deny"`** rejects the request with `403 Forbidden`.
  Use this for strict environments where unverifiable identity is unacceptable.
- **`"ignore"`** (the default) allows the request to proceed using the resolved client IP.
  Use this for best-effort compatibility with mixed proxy environments.

## Evaluation Order

The Network Policy device evaluates rules in the following order:

1. **CIDR allow list**: If the resolved client IP does not match any entry, the request is denied.
2. **Forwarded request check**: If forwarding is disallowed and the request was forwarded, the request is denied.
3. **Forwarded validity check**: If forwarding is allowed but the forwarded identity is invalid, the `on_invalid`
   setting determines whether the request is denied or allowed to proceed.

If any step denies the request, processing stops immediately and no further devices or upstream proxying occurs.

## Path Scoping

By default, the Network Policy device applies to all requests.
The `paths` field restricts enforcement to requests matching one or more path prefixes.

```hcl
network_policy_device {
  enable = true
  cidr_allow = ["10.0.0.0/8"]
  paths = ["/admin/", "/internal/"]

  forwarding {
    allow = false
  }
}
```

Path matching uses prefix semantics with slash-boundary awareness: `/admin` matches `/admin/users` but not `/administrator`.
When `paths` is empty or omitted, the device applies to all requests.
