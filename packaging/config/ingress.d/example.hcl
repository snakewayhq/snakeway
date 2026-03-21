# Example ingress — copy and adapt this file to configure your first service.
# Rename or duplicate it; every *.hcl file in this directory is loaded automatically.

bind = {
  # Listen address. Use "0.0.0.0" to accept connections on all interfaces.
  interface    = "0.0.0.0"
  port         = 80
  enable_http2 = true
}

services = [
  {
    load_balancing_strategy = "round_robin"

    health_check = {
      enable                     = false
      failure_threshold          = 3
      unhealthy_cooldown_seconds = 30
    }

    circuit_breaker = {
      enable_auto_recovery       = true
      failure_threshold          = 5
      open_duration_milliseconds = 30000
      half_open_max_requests     = 1
      success_threshold          = 2
      count_http_5xx_as_failure  = true
    }

    routes = [
      {
        # Match all traffic for these hostnames.
        hosts = ["example.com", "www.example.com"]
        path  = "/"
      }
    ]

    upstreams = [
      {
        weight   = 1
        endpoint = { host = "127.0.0.1", port = 3000 }
      }
    ]
  }
]
