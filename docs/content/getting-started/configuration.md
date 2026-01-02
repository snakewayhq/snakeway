# Configuration

Snakeway cannot run without configuration files.

Directory structure:

```shell
/etc/snakeway/snakeway.toml
/etc/snakeway/routes.d/*.toml
/etc/snakeway/services.d/*.toml
/etc/snakeway/devices.d/*.toml  
```

The good news?

There are CLI commands to help with configuration!

## Generating a config directory

A new configuration directory can be easily generated:

```shell
snakeway config init
```

```shell
snakeway config init /etc/snakeway
```

Which will yield...

```shell                                                                                                                  ✔ 
✔ Initialized Snakeway config in /etc/snakeway
✔ Created:
  - snakeway.toml
  - routes/default.toml
  - services/api.toml
  - devices/identity.toml
  - devices/structured_logging.toml

Next steps:
  snakeway config check
  snakeway run
```

## Validating a config directory

Ahh, but wait! How to tell if the configuration is valid?

```shell
snaekeway config check /etc/snakeway
```

And if everything looks good...

```shell
✔ Config loaded successfully
✔ 3 routes
✔ 1 services
✔ 1 upstreams
✔ 2 devices enabled
```

## Dumping a config directory

Dump the configuration to stdout:

```shell
snakeway config dump /etc/snakeway
```

Will yield this JSON output...

```json
{
  "server": {
    "version": 1,
    "threads": 8,
    "pid_file": "/tmp/snakeway.pid"
  },
  "listeners": [
    {
      "addr": "127.0.0.1:8080",
      "enable_http2": false,
      "enable_admin": false
    }
  ],
  "routes": [
    {
      "path": "/api",
      "target": {
        "type": "service",
        "name": "api"
      },
      "allow_websocket": false,
      "ws_idle_timeout_ms": null,
      "ws_max_connections": null
    },
    {
      "path": "/assets",
      "target": {
        "type": "static",
        "dir": "/var/www/html",
        "index": "index.html",
        "directory_listing": false,
        "static_config": {
          "max_file_size": 10485760,
          "small_file_threshold": 262144,
          "min_gzip_size": 1024,
          "min_brotli_size": 4096,
          "enable_gzip": true,
          "enable_brotli": true
        },
        "cache_policy": {
          "max_age": 3600,
          "public": true,
          "immutable": false
        }
      },
      "allow_websocket": false,
      "ws_idle_timeout_ms": null,
      "ws_max_connections": null
    },
    {
      "path": "/ws",
      "target": {
        "type": "service",
        "name": "api"
      },
      "allow_websocket": true,
      "ws_idle_timeout_ms": 60000,
      "ws_max_connections": 10000
    }
  ],
  "services": {
    "api": {
      "name": "api",
      "strategy": "round_robin",
      "upstream": [
        {
          "url": "http://127.0.0.1:3000",
          "weight": 1
        }
      ],
      "circuit_breaker": {
        "enable_auto_recovery": false,
        "failure_threshold": 0,
        "open_duration_ms": 0,
        "half_open_max_requests": 0,
        "success_threshold": 0,
        "count_http_5xx_as_failure": false
      },
      "health_check": {
        "enable": false,
        "failure_threshold": 0,
        "unhealthy_cooldown_seconds": 0
      }
    }
  },
  "devices": [
    {
      "name": "identity",
      "enabled": true,
      "type": "builtin",
      "builtin": "identity",
      "config": {
        "enable_geoip": false,
        "trusted_proxies": [],
        "ua_engine": "woothee"
      }
    },
    {
      "name": "structured_logging",
      "enabled": true,
      "type": "builtin",
      "builtin": "structured_logging",
      "config": {
        "include_headers": false
      }
    }
  ]
}
```

Or, as yaml:

```shell
snakeway config dump /etc/snakeway --yaml
```

```yaml
server:
  version: 1
  threads: 8
  pid_file: /tmp/snakeway.pid
listeners:
  - addr: 127.0.0.1:8080
    enable_http2: false
    enable_admin: false
routes:
  - path: /api
    target:
      type: service
      name: api
    allow_websocket: false
    ws_idle_timeout_ms: null
    ws_max_connections: null
  - path: /assets
    target:
      type: static
      dir: /var/www/html
      index: index.html
      directory_listing: false
      static_config:
        max_file_size: 10485760
        small_file_threshold: 262144
        min_gzip_size: 1024
        min_brotli_size: 4096
        enable_gzip: true
        enable_brotli: true
      cache_policy:
        max_age: 3600
        public: true
        immutable: false
    allow_websocket: false
    ws_idle_timeout_ms: null
    ws_max_connections: null
  - path: /ws
    target:
      type: service
      name: api
    allow_websocket: true
    ws_idle_timeout_ms: 60000
    ws_max_connections: 10000
services:
  api:
    name: api
    strategy: round_robin
    upstream:
      - url: http://127.0.0.1:3000
        weight: 1
    circuit_breaker:
      enable_auto_recovery: false
      failure_threshold: 0
      open_duration_ms: 0
      half_open_max_requests: 0
      success_threshold: 0
      count_http_5xx_as_failure: false
    health_check:
      enable: false
      failure_threshold: 0
      unhealthy_cooldown_seconds: 0
devices:
  - name: identity
    enabled: true
    type: builtin
    builtin: identity
    config:
      enable_geoip: false
      trusted_proxies: [ ]
      ua_engine: woothee
  - name: structured_logging
    enabled: true
    type: builtin
    builtin: structured_logging
    config:
      include_headers: false
```

