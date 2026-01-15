---
title: Your First Proxy
---


The best way to understand Snakeway is to see it in action. In this guide, we'll walk through setting up a minimal proxy
that forwards traffic to a public API.

### 1. Initialize Your Configuration

First, create a new directory to hold your Snakeway configuration:

```shell
snakeway config init ./my-proxy
```

### 2. Configure the Server

The entrypoint config file should exist: `./my-proxy/snakeway.hcl`.

It should have something that looks like this:

```hcl
server {
  version = 1
}

include {
  ingress = "ingress.d/*.hcl"
  devices = "devices.d/*.hcl"
}
```

### 3. Define the Ingress

Next, we'll define the ingress we want to proxy to. Create `my-proxy/ingress.d/httpbin.hcl`:

```hcl
bind = {
  interface = "127.0.0.1"
  port      = 8080
}

services = [
  {
    load_balancing_strategy = "round_robin"

    routes = [
      {
        path = "/get"
      }
    ]

    upstreams = [
      {
        addr = "httpbin.org:443"
      }
    ]
  }
]
```

This tells Snakeway that there is a service that handles requests to `/get` and forwards them to `httpbin.org`.

With this configuration, any request sent to `http://localhost:8080/get` will be proxied to `https://httpbin.org/get`.

### 5. Launch the Proxy

Run Snakeway, pointing it to your new configuration directory:

```bash
snakeway run --config ./my-proxy
```

### 6. Verify with Curl

Finally, open a new terminal and send a request to your local proxy:

```bash
curl -i http://localhost:8080/get
```

You should see a successful response from `httpbin.org`, served through your local Snakeway instance!

```http
HTTP/1.1 200 OK
Content-Type: application/json
...
{
  "args": {},
  "headers": {
    "Host": "httpbin.org",
    ...
  },
  "url": "https://httpbin.org/get"
}
```

Congratulations! You've just configured and launched your first Snakeway proxy. From here, you can begin exploring more
advanced features like [Devices](/devices/overview) and [Static File Serving](/getting-started/static-files).
