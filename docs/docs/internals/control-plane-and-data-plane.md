---
title: Control Plane and Data Plane
---

Snakeway separates execution into two runtime domains: the control plane
and the data plane. This separation ensures that operational and
lifecycle management tasks do not interfere with request processing
performance. The model follows patterns commonly used in
high-performance proxies and network infrastructure software.

## Architectural Overview

The system is composed of two independent runtime contexts with
different responsibilities and constraints.

The control plane coordinates system lifecycle and background services.
The data plane executes the request path and must maintain predictable
latency characteristics under load.

All long-running asynchronous tasks, external network clients, and
operational services run in the control plane. Request handling and
routing logic execute exclusively in the data plane.

This boundary prevents operational tasks from introducing blocking
behavior or unpredictable scheduling into the request path.

## Control Plane

The control plane manages the operational state of the proxy. It is
responsible for tasks that are not directly involved in processing
individual client requests.

Examples of control plane responsibilities include:

- configuration loading and validation
- runtime configuration reload
- ACME certificate issuance and renewal
- certificate store management
- telemetry exporters and observability pipelines
- background maintenance tasks

These components rely on asynchronous runtimes and may perform network
IO, filesystem operations, or remote API calls. Since these activities
can involve unpredictable latency, they are intentionally isolated from
the request processing path.

The control plane runs on a dedicated Tokio runtime that is created
during startup and persists for the lifetime of the process.

## Data Plane

The data plane is responsible for handling incoming traffic and proxying
requests to upstream services. It operates on Pingora worker threads and
represents the performance-critical path of the system.

Responsibilities of the data plane include:

- accepting inbound connections
- parsing and validating requests
- routing requests to upstream services
- executing the device pipeline
- forwarding requests and responses

Data plane execution must avoid blocking operations, long allocations,
or unpredictable scheduling behavior. Code running in this context
should be deterministic and minimal in overhead.

Any operation that could block or require asynchronous coordination must
be delegated to the control plane.

## Observability Placement

Observability infrastructure follows the same separation model.

Telemetry exporters and trace processors run in the control plane
runtime because they involve network communication with external
telemetry backends. Exporting telemetry directly from the request path
would introduce unacceptable latency and backpressure risk.

The data plane only emits structured logs and tracing events through the
tracing instrumentation layer. These events are processed by the
subscriber pipeline and exported asynchronously by the control plane.

This design ensures that request instrumentation remains lightweight
while still providing full observability into system behavior.

## Operational Benefits

Separating control plane and data plane execution provides several
practical advantages:

- predictable request latency under load
- isolation of operational services from traffic processing
- simpler reasoning about concurrency boundaries
- safer integration of asynchronous background tasks

The result is an architecture that maintains high throughput while
supporting operational features such as certificate automation,
telemetry export, and runtime configuration management.

## Summary

Snakeway enforces a clear separation between operational infrastructure
and request execution. The control plane manages system lifecycle and
asynchronous services, while the data plane remains focused on
deterministic request handling within Pingora worker threads. This model
preserves performance characteristics while enabling advanced
operational capabilities.

