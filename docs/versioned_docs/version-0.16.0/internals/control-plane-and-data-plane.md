---
title: Control Plane and Data Plane
---

Snakeway runs two independent runtime contexts, the control plane and the data plane.

## Architectural overview

The control plane coordinates system lifecycle and background services.
The data plane executes the request path.

All long-running asynchronous tasks, external network clients, and operational services run in the control plane.
Request handling and routing logic execute exclusively in the data plane.
An operational task therefore cannot block a worker thread or add scheduling delay to a request in flight.

## Control plane

The control plane manages the operational state of the proxy.
It is responsible for tasks that are not directly involved in processing individual client requests.

Examples of control plane responsibilities include:

- configuration loading and validation
- runtime configuration reload
- ACME certificate issuance and renewal
- certificate store management
- telemetry exporters and observability pipelines
- background maintenance tasks

These components rely on asynchronous runtimes and may perform network IO, filesystem operations, or remote API calls.
Since these activities can involve unpredictable latency, they are intentionally isolated from the request processing path.

The control plane runs on a dedicated Tokio runtime that is created during startup and persists for the lifetime of the process.

## Data plane

The data plane is responsible for handling incoming traffic and proxying requests to upstream services.
It operates on Pingora worker threads and represents the performance-critical path of the system.

Responsibilities of the data plane include:

- accepting inbound connections
- parsing and validating requests
- routing requests to upstream services
- executing the device pipeline
- forwarding requests and responses

Data plane execution must avoid blocking operations, long allocations, or unpredictable scheduling behavior.
Code running in this context should be deterministic and minimal in overhead.

Any operation that could block or require asynchronous coordination must be delegated to the control plane.

## Observability placement

Observability infrastructure follows the same separation model.

Telemetry exporters and trace processors run in the control plane runtime because they involve network communication with external telemetry backends.
Exporting telemetry directly from the request path would introduce unacceptable latency and backpressure risk.

The data plane only emits structured logs and tracing events through the tracing instrumentation layer.
These events are processed by the subscriber pipeline and exported asynchronously by the control plane.
