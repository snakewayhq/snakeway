---
title: Circuit Breaker
---

The circuit breaker protects your services by stopping traffic to failing upstreams.
It is configured inside a [service](services.md) block.

## State Transitions

The circuit breaker operates in three states:

1. **Closed** (normal operation).
   Requests flow to the upstream.
   When consecutive failures reach the `failure_threshold`, the circuit transitions to Open.
2. **Open** (blocking).
   All requests are rejected immediately.
   After `open_duration_milliseconds` elapses, the circuit transitions to HalfOpen.
3. **HalfOpen** (probing).
   A limited number of probe requests (`half_open_max_requests`) are allowed through.
   If `success_threshold` probes succeed, the circuit closes again.
   If any probe fails, the circuit reopens.

```hcl
circuit_breaker = {
  # Enable or disable the circuit breaker.
  enable                     = true

  # Allow automatic recovery after the open duration.
  enable_auto_recovery       = false

  # Failures before tripping to Open.
  failure_threshold          = 3

  # Milliseconds the circuit stays Open.
  open_duration_milliseconds = 10000

  # Probe requests allowed in HalfOpen.
  half_open_max_requests     = 1

  # Successful probes needed to close the circuit.
  success_threshold          = 2

  # Count HTTP 5xx responses as failures.
  count_http_5xx_as_failure  = false
}
```

## Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `enable` | `boolean` | `true` | Enable or disable the circuit breaker. |
| `enable_auto_recovery` | `boolean` | `false` | Allow the circuit breaker to automatically recover after the open duration. |
| `failure_threshold` | `integer` | `5` | Number of consecutive failures (transport errors or 5xx) in the Closed state before tripping the circuit to Open. |
| `open_duration_milliseconds` | `integer` | `10000` | How long, in milliseconds, the circuit remains Open before transitioning to HalfOpen. |
| `half_open_max_requests` | `integer` | `1` | How many simultaneous probe requests are allowed while in the HalfOpen state. |
| `success_threshold` | `integer` | `2` | How many successful probes are required in HalfOpen to close the circuit again. |
| `count_http_5xx_as_failure` | `boolean` | `true` | Whether HTTP 5xx responses from the upstream count as failures for the circuit breaker. |
