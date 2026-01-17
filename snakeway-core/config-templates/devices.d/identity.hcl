identity_device = {
  enable = true

  enable_geoip = false
  trusted_proxies = []
  # Define the available databases (not included with Snakeway)...
  # geoip_city_db            = "/path/to/city.mmdb"
  # geoip_isp_db             = "/path/to/isp.mmdb"
  # geoip_connection_type_db = "/path/to/connection_type.mmdb"

  enable_user_agent = true
  ua_engine         = "woothee"
}