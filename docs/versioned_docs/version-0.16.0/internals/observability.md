---
title: Observability
---

The observability subsystem emits three kinds of signal:

- structured logs suitable for machine ingestion
- distributed tracing using OpenTelemetry
- metrics exported over OTLP

Exporter lifecycles and the asynchronous telemetry tasks run on the control plane runtime.
Data plane code emits events and spans and hands them off, so no export work happens on the request path.
Every provider is initialized explicitly at a fixed point in startup rather than on first use.

## Design principles

### Structured logging

Snakeway uses the `tracing` ecosystem for structured logging.
Logs are emitted as JSON events with flattened fields to make ingestion by log aggregation systems straightforward.

Logs are intended to represent operational state transitions such as:

- startup and shutdown events
- configuration reload activity
- certificate automation lifecycle
- device pipeline execution outcomes
- upstream proxy interactions

The logging system supports two output modes:

- standard output for container environments
- rolling file logs when `SNAKEWAY_LOG_DIR` is configured

File logging uses a non-blocking writer to ensure disk IO does not affect the request path.

### Distributed tracing

Distributed tracing is implemented through OpenTelemetry and the `tracing-opentelemetry` integration layer.

The OpenTelemetry pipeline is configured through the Snakeway runtime configuration and includes:

- OTLP exporter endpoint
- service metadata such as `service.name` and `service.version`
- sampling strategy

Tracing spans originate from the `tracing` instrumentation already used for structured logging.
The subscriber bridges these spans into the OpenTelemetry exporter.

The exporter operates using a batch processor that runs on the control plane runtime.
This avoids blocking request processing while traces are exported.

### Runtime metadata

Snakeway attaches resource metadata to every exported trace.
The metadata follows OpenTelemetry semantic conventions and includes service attributes such as:

- service name
- service version
- instance identifier derived from the system hostname

This information allows telemetry backends to distinguish between instances in multi-node deployments.

### Sampling

Snakeway uses a parent-based sampling model.
When an incoming request carries a sampled W3C Trace Context, the proxy always honors that decision and samples the request.
When no parent context is present, the `sampling_ratio` setting determines what fraction of root traces are sampled using a deterministic trace-ID-ratio algorithm.

The default `sampling_ratio` of `1.0` samples all root traces.
Setting it to a lower value (e.g., `0.1` for 10%) reduces trace volume in high-traffic deployments while preserving complete traces for every sampled request.

### Request instrumentation

A root `request` span is created for every proxied request inside the Pingora `request_filter` hook.
The span carries the following fields:

- `http.method`
- `http.host`
- `http.path`
- `client.ip`
- `request.id`
- `listener`
- `route`

When the incoming request includes W3C Trace Context headers (`traceparent` / `tracestate`), the request span is automatically parented to the upstream trace.
The same trace context is injected into the request sent to the upstream service, so Snakeway appears as an intermediate span in a distributed trace.

Child spans are created for each major phase of request processing:

- `routing` covers on-request device pipeline execution and route matching
- `upstream_selection` covers the traffic decision, upstream peer creation, and circuit breaker admission
- `upstream_request` covers the before-proxy device pipeline, header mutation, and trace context injection
- `upstream_response` covers the after-proxy device pipeline and response status mutation
- `response` covers the on-response device pipeline and upstream outcome determination

### Log export

When OpenTelemetry is enabled, log events emitted through the `tracing` framework are also exported to the configured OTLP endpoint.
The `opentelemetry-appender-tracing` bridge converts `tracing` events into OpenTelemetry log records and sends them via a batch processor on the control plane runtime.

An internal filter suppresses noisy crates (`pingora`, `tonic`, `h2`, `reqwest`) so that only application-level events are exported.

### Metrics

When OpenTelemetry is enabled, Snakeway exports the following metric instruments via the OTLP exporter:

| Metric                              | Type           | Attributes                     |
|-------------------------------------|----------------|--------------------------------|
| `snakeway.http.requests`            | Counter        | method, status, service, route |
| `snakeway.http.request.duration`    | Histogram (ms) | service, upstream              |
| `snakeway.http.errors`              | Counter        | service, upstream, error.type  |
| `snakeway.upstream.active_requests` | Gauge          | service, upstream              |
| `snakeway.upstream.health`          | Gauge (0/1)    | service, upstream              |
| `snakeway.circuit_breaker.state`    | Gauge (0/1/2)  | service, upstream              |

All per-request metrics are recorded in the Pingora `logging` hook, which runs last and has access to the complete request context including service, upstream, outcome, and timing.

Circuit breaker state values:

- `0` is closed (healthy)
- `1` is open (tripped)
- `2` is half-open (recovery testing)

### Shutdown behavior

The OpenTelemetry tracer, logger, and meter providers are stored globally so that exporters can flush pending data during shutdown.
This ensures that traces and logs generated during the final moments of process execution are not lost.

Graceful shutdown hooks trigger provider shutdown before the process exits.
