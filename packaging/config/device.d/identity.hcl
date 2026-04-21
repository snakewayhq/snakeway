identity_device = {
  enable = false

  # Where is the client located and what type of connection do they have?
  enable_geoip = false
  trusted_proxies = []

  # geoip_city_db = "path/to/city.mmdb"
  # geoip_isp_db  = "path/to/isp.mmdb"
  # geoip_connection_type_db = "path/to/connection_type.mmdb"

  # Which device is the client connecting from?
  enable_user_agent = false
  ua_engine         = "woothee"
  # ua_parser_regexes = "path/to/regexes.yaml"
}
