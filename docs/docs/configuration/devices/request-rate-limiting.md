---
title: Request Rate Limiting Device
---

The **Request Rate Limiting** device is a builtin Snakeway device that limits the **rate of incoming HTTP requests** on
a per-client basis.

It is intended to protect upstream services from sustained request floods while allowing short-lived traffic bursts to
pass.

:::note
The **Identity** device is required by the **Request Rate Limiting** device.

The **Request Rate Limiting** device operates after the **Identity** device in the device pipeline
:::

## Configuration Example

This example limits clients to an average of **20 requests per second**, measured over a **5-second window**.

```hcl
request_rate_limiting_device = {
  enable = true

  max_requests_per_second = 20
  window_seconds          = 5
}
```

## How Request Rate Limiting Works

Requests are evaluated based on their **average request rate**, measured in requests per second and calculated over a
rolling time window.

Key properties:

* Rate limits are enforced **per client**.
* Short bursts may be allowed.
* Sustained high request rates are rejected.
* Limits automatically recover once traffic subsides.
