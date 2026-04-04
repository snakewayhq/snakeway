---
title: Observability
---

Snakeway provides built‑in observability primitives intended for
production deployments of a high‑performance edge proxy. The
observability subsystem is designed to expose operational signals while
keeping overhead predictable and minimal in the request hot path.

The system focuses on three goals:

- structured logs suitable for machine ingestion
- distributed tracing using OpenTelemetry
- a separation between control plane instrumentation and data plane
  execution

The implementation favors explicit initialization and deterministic
startup ordering to avoid hidden runtime behavior.

## Summary

The observability subsystem in Snakeway is intentionally minimal in the
request hot path while still exposing the signals required to operate
the proxy in production environments. Control plane components manage
exporter lifecycles and asynchronous telemetry tasks, while data plane
code emits structured events and tracing spans with minimal overhead.

## Design Principles

### Structured Logging

Snakeway uses the `tracing` ecosystem for structured logging. Logs are
emitted as JSON events with flattened fields to make ingestion by log
aggregation systems straightforward.

Logs are intended to represent operational state transitions such as:

- startup and shutdown events
- configuration reload activity
- certificate automation lifecycle
- device pipeline execution outcomes
- upstream proxy interactions

The logging system supports two output modes:

- standard output for container environments
- rolling file logs when `SNAKEWAY_LOG_DIR` is configured

File logging uses a non‑blocking writer to ensure disk IO does not affect the request path.

### Distributed Tracing

Distributed tracing is implemented through OpenTelemetry and the
`tracing-opentelemetry` integration layer.

The OpenTelemetry pipeline is configured through the Snakeway runtime
configuration and includes:

- OTLP exporter endpoint
- service metadata such as `service.name` and `service.version`
- sampling strategy

Tracing spans originate from the `tracing` instrumentation already used
for structured logging. The subscriber bridges these spans into the
OpenTelemetry exporter.

The exporter operates using a batch processor that runs on the control
plane runtime. This avoids blocking request processing while traces are
exported.

### Runtime Metadata

Snakeway attaches resource metadata to every exported trace. The
metadata follows OpenTelemetry semantic conventions and includes service
attributes such as:

- service name
- service version
- instance identifier derived from the system hostname

This information allows telemetry backends to distinguish between
instances in multi‑node deployments.

### Sampling

The current implementation supports a parent‑based sampling model. The
sampling decision propagates across nested spans so that trace
hierarchies remain coherent.

Additional sampling strategies may be introduced later, but the default
configuration ensures that traces remain complete once a root span has
been sampled.

### Request Instrumentation

A root `request` span is created for every proxied request inside the
Pingora `request_filter` hook. The span carries the following fields:

- `http.method`
- `http.host`
- `http.path`
- `client.ip`
- `request.id`
- `listener`
- `route`

When the incoming request includes W3C Trace Context headers
(`traceparent` / `tracestate`), the request span is automatically
parented to the upstream trace. The same trace context is injected into
the request sent to the upstream service, so Snakeway appears as an
intermediate span in a distributed trace.

Child spans are created for each major phase of request processing:

- `routing` -- on-request device pipeline execution and route matching
- `upstream_selection` -- traffic decision, upstream peer creation, and circuit breaker admission
- `upstream_request` -- before-proxy device pipeline, header mutation, and trace context injection
- `upstream_response` -- after-proxy device pipeline and response status mutation
- `response` -- on-response device pipeline and upstream outcome determination

### Log Export

When OpenTelemetry is enabled, log events emitted through the `tracing`
framework are also exported to the configured OTLP endpoint. The
`opentelemetry-appender-tracing` bridge converts `tracing` events into
OpenTelemetry log records and sends them via a batch processor on the
control plane runtime.

An internal filter suppresses noisy crates (`pingora`, `tonic`, `h2`,
`reqwest`) so that only application-level events are exported.

### Shutdown Behavior

The OpenTelemetry tracer, logger, and meter providers are stored
globally so that exporters can flush pending data during shutdown. This
ensures that traces and logs generated during the final moments of
process execution are not lost.

Graceful shutdown hooks trigger provider shutdown before the process
exits.
