---
title: Identity Device
---

The **Identity device** resolves a canonical, request-scoped view of the client making each request.
It determines the true client IP address (accounting for `X-Forwarded-For` headers and trusted proxy chains), enriches the request with GeoIP data, and parses the `User-Agent` header into structured fields.
Identity resolution runs once, early in the request lifecycle, and stores the result in a typed extension on the request context so that downstream devices such as [Network Policy](/docs/configuration/devices/network-policy), [Rate Limiting](/docs/configuration/devices/request-rate-limiting), and [Structured Logging](/docs/configuration/devices/structured-logging/) can consume it without re-parsing headers.

## Configuration Example

```hcl
identity_device = {
  enable = true

  # IP trust
  trusted_proxies = ["10.0.0.0/8", "172.16.0.0/12"]
  max_x_forwarded_for_length = 1024

  # GeoIP enrichment
  enable_geoip  = true
  geoip_city_db = "/var/lib/snakeway/mmdb/city.mmdb"
  geoip_isp_db  = "/var/lib/snakeway/mmdb/isp.mmdb"
  geoip_connection_type_db = "/var/lib/snakeway/mmdb/connection_type.mmdb"

  # User-Agent parsing
  enable_user_agent     = true
  ua_engine             = "woothee"
  # woothee does not use regexes, but if using the `uaparser` 
  # the embedded regex file can be overridden. 
  # ua_parser_regexes       = "/var/lib/snakeway/regexes.yaml"
  max_user_agent_length = 2048
}
```

## Field Reference

| Field                        | Type            | Default     | Description                                                                                                                                                                                               |
|------------------------------|-----------------|-------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `enable`                     | bool            | `false`     | Enables the Identity device.                                                                                                                                                                              |
| `trusted_proxies`            | list of strings | `[]`        | CIDR ranges whose `X-Forwarded-For` entries are trusted for IP resolution.                                                                                                                                |
| `max_x_forwarded_for_length` | integer         | `1024`      | Maximum byte length of the `X-Forwarded-For` header that will be parsed. Headers exceeding this limit are ignored.                                                                                        |
| `enable_geoip`               | bool            | `false`     | Enables GeoIP lookups against MaxMind MMDB databases.                                                                                                                                                     |
| `geoip_city_db`              | string (path)   | none        | Path to a MaxMind City MMDB file. Provides country, region, and city-level geolocation.                                                                                                                   |
| `geoip_isp_db`               | string (path)   | none        | Path to a MaxMind ISP MMDB file. Provides ASN, ASO, and ISP metadata.                                                                                                                                     |
| `geoip_connection_type_db`   | string (path)   | none        | Path to a MaxMind Connection Type MMDB file. Identifies Cable/DSL, Cellular, Corporate, or Satellite connections.                                                                                         |
| `enable_user_agent`          | bool            | `false`     | Enables user-agent string parsing.                                                                                                                                                                        |
| `ua_engine`                  | string          | `"woothee"` | Parser engine. `"woothee"` is fast and policy-based. `"uaparser"` is regex-based, slower, but potentially more accurate.                                                                                  |
| `ua_parser_regexes`          | string (path)   | none        | Path to a custom [ua-parser `regexes.yaml`](https://github.com/ua-parser/uap-core) file. When omitted, the bundled regexes compiled into the binary are used. Only applies when `ua_engine = "uaparser"`. |
| `max_user_agent_length`      | integer         | `2048`      | Maximum byte length of the `User-Agent` header that will be parsed. Longer values are ignored to prevent abuse.                                                                                           |

:::note
GeoIP databases are not included with Snakeway.
You must obtain them separately from [MaxMind](https://www.maxmind.com/) and provide the file paths in your configuration.
:::

## Client IP Resolution

The Identity device resolves the true client IP by walking the `X-Forwarded-For` header from right to left, stripping entries that match `trusted_proxies`.
The first non-trusted IP is treated as the client address.
If no `X-Forwarded-For` header is present, or if all entries are trusted, the TCP peer address is used.

### Behind a CDN

When Snakeway sits behind a CDN such as Cloudflare or Fastly, add the CDN's IP ranges to `trusted_proxies` so that the CDN's own address is skipped during resolution.

```hcl
trusted_proxies = ["173.245.48.0/20", "103.21.244.0/22"]
```

### Behind a Load Balancer

For internal load balancers (AWS ALB, HAProxy, Nginx), include the load balancer's subnet.

```hcl
trusted_proxies = ["10.0.0.0/8"]
```

### Direct Client Connections

If Snakeway receives traffic directly from clients without any intermediate proxies, leave `trusted_proxies` empty.
The TCP peer address will be used as the client IP, and any `X-Forwarded-For` header present in the request will be treated as untrusted.

## How Downstream Devices Access Identity

The resolved identity is stored as a `ClientIdentity` struct in the request context's typed extension map.
Downstream devices read this struct directly rather than re-parsing headers.
This ensures that every device in the pipeline sees the same client IP, geolocation, and user-agent classification, regardless of ordering or configuration differences.

For example, the [Network Policy device](/docs/configuration/devices/network-policy) checks `ClientIdentity.client_ip` against its CIDR allowlist, and the [Structured Logging device](/docs/configuration/devices/structured-logging/) selects fields from `ClientIdentity` based on its `identity_fields` configuration.
