---
title: Identity Device
---

The **Identity** builtin device provides a canonical, request-scoped view of the client making a request. It resolves
client identity **once**, early in the request lifecycle, and exposes it to downstream devices via a typed extension on
the request context.

## Configuration Example

```hcl
identity_device = {
  enable = true

  trusted_proxies = ["10.0.0.0/8"]

  # Enable GeoIP...
  enable_geoip = true

  # If GeoIP is enabled, 
  # set one ore more MMDB databases (not included with Snakeway)...
  geoip_city_db = "/path/to/city.mmdb"
  geoip_isp_db  = "/path/to/isp.mmdb"
  geoip_connection_type_db = "/path/to/connection_type.mmdb"

  # Enable user agent enrichment...
  enable_user_agent = true
  # If user-agent parsing is enabled,
  # set an engine...
  ua_engine         = "woothee" # or "uaparser"
}
```

## User-Agent Parsing

The default user-agent is [Woothee](https://woothee.github.io/).
Woothee is a fast policy-based user agent parser.

An alternate regex based `ua_engine` is available called **UA Parser** ("uaparser").
UA Parser is slower than Woothee, but possibly more accurate.

