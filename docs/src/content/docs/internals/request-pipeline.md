# Request Pipeline

Understanding the journey of a request through Snakeway is key to building effective traffic logic. The pipeline is
designed to be deterministic, with clearly defined stages for both processing and extension.

### 1. Acceptance and Parsing

The journey begins when a client establishes a connection to one of Snakeway's listeners. Pingora accepts the connection
and begins parsing the incoming HTTP stream into request headers and, optionally, a body.

### 2. Initial Device Hook: `on_request`

As soon as the headers are parsed, the request enters the Device pipeline. The `on_request` hook is called for all
enabled devices in the order they are defined.

- **Purpose**: Early inspection and modification. This is where the `Identity` device typically runs to establish client
  context.
- **Decision**: Devices can choose to `Continue` the pipeline or `Respond` early (short-circuiting).

### 3. Routing

Once the initial device hooks are complete, Snakeway's router matches the request path against the configured routes.

- **Service Route**: If the path matches a service route, Snakeway prepares to proxy the request to the associated
  upstream service.
- **Static Route**: If the path matches a static route, Snakeway hands the request over to the high-performance static
  file handler.

### 4. Final Preparation: `before_proxy`

For service routes, Snakeway performs a final pass through the device pipeline using the `before_proxy` hook.

- **Purpose**: Final enrichment and routing decisions. This is the last chance to add headers (e.g., `X-Forwarded-For`)
  before the request hits application code.

### 5. Upstream Forwarding

Snakeway selects an upstream server based on the service's load balancing strategy and forwards the modified request. It
then waits for the upstream to begin sending the response.

### 6. Upstream Response: `after_proxy`

When the upstream response headers are received, the pipeline continues with the `after_proxy` hook.

- **Purpose**: Inspecting the upstream's response. This is often used for logging or reacting to specific status codes (
  e.g., triggering a circuit breaker).

### 7. Final Response: `on_response`

Just before the response is sent back to the client, the `on_response` hook is executed.

- **Purpose**: Final response modification. You might use this to add security headers or strip sensitive information
  before the client sees it.

### 8. Delivery

Finally, the response is delivered to the client. The connection is then either closed or returned to the pool for
reuse, depending on the HTTP version and configuration.
