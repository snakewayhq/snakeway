identity_device = {
  enable = true

  enable_geoip = true

  geoip_db = "fixtures/geoip/dbip-country-lite-2025-12.mmdb"

  trusted_proxies = ["127.0.0.1/32"]

  enable_user_agent = true

  ua_engine = "woothee"
}