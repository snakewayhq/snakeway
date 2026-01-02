# Your First Proxy

The best way to understand Snakeway is to see it in action. In this guide, we'll walk through setting up a minimal proxy
that forwards traffic to a public API.

### 1. Initialize Your Configuration

First, create a new directory to hold your Snakeway configuration:

```shell
snakeway config init ./my-proxy
```

### 2. Configure the Server

The entrypoint config file should exist: `./my-proxy/snakeway.toml`.

It should have something that looks like this:

```toml
[server]
version = 1

[[listener]]
addr = "127.0.0.1:8080"

[include]
routes = "routes.d/*.toml"
services = "services.d/*.toml"
devices = "devices.d/*.toml"
```

### 3. Define the Upstream Service

Next, we'll define the service we want to proxy to. Create `my-proxy/services.d/httpbin.toml`:

```toml
[[service]]
name = "httpbin"
strategy = "round_robin"

[[service.upstream]]
url = "https://httpbin.org"
```

This tells Snakeway that there is a service named `httpbin` located at `https://httpbin.org`.

### 4. Create a Route

Now, let's map an incoming request path to our service. Create `my-proxy/routes.d/api.toml`:

```toml
[[service_route]]
path = "/get"
service = "httpbin"
```

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
