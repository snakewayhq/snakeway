# Performance Metrics

Monitoring the performance and health of your proxy is critical for maintaining a reliable service. Snakeway provides
real-time metrics through its Admin API, giving you visibility into traffic patterns and upstream health.

### The Stats API

The primary way to collect metrics from Snakeway is through the `/admin/stats` endpoint. This endpoint provides counters
and gauges for various system activities.

#### Sample Metrics Output

```json
{
  "api": {
    "active_requests": 5,
    "total_failures": 12,
    "total_requests": 1500,
    "total_successes": 1488
  },
  "static": {
    "active_requests": 0,
    "total_failures": 0,
    "total_requests": 450,
    "total_successes": 450
  }
}
```

### Metrics per Service

Stats are grouped by the service name defined in your configuration. For each service, Snakeway tracks:

- **`active_requests`**: The number of requests currently being processed.
- **`total_requests`**: The cumulative number of requests handled by the service since the last restart or reload.
- **`total_successes`**: The number of requests that resulted in a successful response (typically 2xx and 3xx status
  codes).
- **`total_failures`**: The number of requests that resulted in an error (typically 4xx and 5xx status codes, or
  connection failures).

### Integration with External Tools

While the Admin API provides raw metrics in JSON format, you can easily integrate Snakeway with external monitoring
systems:

- **Prometheus**: You can use a simple sidecar or "exporter" script to poll the `/admin/stats` endpoint and convert the
  JSON data into Prometheus-compatible metrics.
- **CloudWatch/Datadog**: Similarly, custom scripts can be used to push Snakeway metrics to your preferred cloud
  monitoring platform.

### Future Roadmap

We are planning to include native support for Prometheus and OpenTelemetry in future versions of Snakeway. This will
allow for even easier integration with modern observability stacks, including automatic export of histograms for request
latency and percentile tracking.
