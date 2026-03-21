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

File logging uses a non‑blocking writer to ensure the request path is
not affected by disk IO.

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

Request processing is instrumented through spans created within the
Pingora gateway implementation. A typical trace hierarchy represents the
full request lifecycle:

- request span
- routing span
- device pipeline spans
- upstream proxy span

The device pipeline model allows individual devices to emit spans
describing enrichment, filtering, or policy evaluation performed on the
request.

### Shutdown Behavior

The OpenTelemetry tracer provider is stored globally so that the
exporter can flush pending spans during shutdown. This ensures that
traces generated during the final moments of process execution are not
lost.

Graceful shutdown hooks trigger exporter shutdown before the process
exits.
