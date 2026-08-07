---
title: Connection Filter
---

The `connection_filter` block restricts which clients may connect based on their source IP address.
It is nested inside the [bind](bind.md) block and contains three sub-sections: `cidr`, `ip_family`, and `on_no_peer_addr`.

```hcl
connection_filter = {
  cidr = {
    # CIDR blocks to allow. If empty, all connections are allowed.
    allow = []

    # CIDR blocks to deny. If empty, no connections are denied.
    deny  = [
      "10.0.0.0/8",
      "192.168.0.0/16"
    ]
  }

  ip_family = {
    # Allow IPv4 connections.
    ipv4 = true

    # Allow IPv6 connections.
    ipv6 = false
  }

  # Behavior when a connection has no peer address.
  on_no_peer_addr = "allow"
}
```

## CIDR Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `cidr.allow` | `list(string)` | `[]` | CIDR blocks to allow connections from. If empty, all connections are allowed. |
| `cidr.deny` | `list(string)` | `[]` | CIDR blocks to deny connections from. If empty, no connections are denied. |

:::note
The `deny` list takes precedence over the `allow` list.
:::

## IP Family Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `ip_family.ipv4` | `boolean` | `false` | If `true`, IPv4 connections are allowed. |
| `ip_family.ipv6` | `boolean` | `false` | If `true`, IPv6 connections are allowed. |

:::note
At least one of `ipv4` or `ipv6` must be `true`.
A config error will be reported if both are `false`.
:::

## On No Peer Address

| Field | Type | Default | Description |
|---|---|---|---|
| `on_no_peer_addr` | `string` | `"allow"` | Controls the behavior when a connection arrives without a peer address. Accepts `"allow"` or `"deny"`. The default is permissive. Any connection is allowed. |
