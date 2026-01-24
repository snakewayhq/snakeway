bind = {
  interface    = "127.0.0.1"
  port         = 8080
  enable_http2 = false
}

services = [
  {
    load_balancing_strategy = "round_robin"

    health_check = {
      enable                     = false
      failure_threshold          = 3
      unhealthy_cooldown_seconds = 10
    }

    circuit_breaker = {
      enable_auto_recovery       = false
      failure_threshold          = 3
      open_duration_milliseconds = 10000
      half_open_max_requests     = 1
      success_threshold          = 2
      count_http_5xx_as_failure  = false
    }

    routes = [
      {
        path = "/api"
      },
      {
        path               = "/ws"
        enable_websocket   = true
        ws_max_connections = 10000
      }
    ]

    upstreams = [
      {
        weight = 1
        endpoint = { host = "127.0.0.1", port = 9001 }
      },
      {
        weight = 1
        endpoint = { host = "127.0.0.1", port = 9002 }
      },
    ]
  }
]

static_files = [
  {
    routes = [
      {
        path              = "/assets"
        file_dir          = "/var/www/html"
        index             = "index.html"
        directory_listing = false
        max_file_size     = 1048576

        compression = {
          enable_gzip          = false
          small_file_threshold = 104857
          min_gzip_size        = 1024
          enable_brotli        = false
          min_brotli_size      = 4096
        }

        cache_policy = {
          max_age_seconds = 60
          public          = true
          immutable       = false
        }
      }
    ]
  }
]
