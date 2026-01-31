bind = {
  interface    = "127.0.0.1"
  port         = 8443
  enable_http2 = false

  redirect_http_to_https = {
    port   = 8080
    status = 308
  }

  tls = {
    cert = "./integration-tests/certs/server.pem"
    key  = "./integration-tests/certs/server.key"
  }

  connection_filter = {
    enabled = true

    cidr = {
      allow = []
      deny = [
        "10.0.0.0/8",
        "192.168.0.0/16"
      ]
    }

    ip_family = {
      ipv4 = true
      ipv6 = false
    }

    on_no_peer_addr = "allow" # allow | deny
  }
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
        endpoint = { interface = "127.0.0.1", port = 3443 }
      },
      {
        weight = 1
        endpoint = { interface = "127.0.0.1", port = 3444 }
      },
      {
        weight = 1
        sock   = "/tmp/snakeway-http-1.sock"
      }
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
