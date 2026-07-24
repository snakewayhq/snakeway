---
title: JWT Auth WASM Device
---

The **JWT Auth device** validates an HMAC-SHA256 (HS256) bearer token on each request, injects the caller identity into
upstream headers, and rejects unauthenticated requests with a generic 401.

:::note
This is an example device shipped in the Snakeway repository (`crates/snakeway-jwt-auth-device`), not a supported
product surface.
Use it as a reference and a starting point for your own auth device.
The WIT interface it builds against may change in future releases.
:::

## What It Does

On the `on_request` hook the device:

- Reads the `Authorization: Bearer <token>` header and validates the token.
- On success, injects `X-User-Id` (and optionally `X-Tenant-Id`) from the token claims, strips the `Authorization`
  header, and forwards the request.
- On failure, returns a generic `401` and does not forward the request.
- Returns a generic `500` when the device itself is misconfigured.

The device only asserts identity that it derives from a validated token.
It always removes any client-supplied `X-User-Id` and `X-Tenant-Id` before setting its own, so a client cannot spoof
identity by sending those headers.

## Building

```shell
cargo component build --release --target wasm32-unknown-unknown -p snakeway-jwt-auth-device
cp target/wasm32-unknown-unknown/release/snakeway_jwt_auth_device.wasm /etc/snakeway/devices/
```

## Configuration Example

```hcl
wasm_devices = [
  {
    name        = "jwt-auth"
    enable      = true
    path        = "/etc/snakeway/devices/snakeway_jwt_auth_device.wasm"
    fail_policy = "closed"
    hooks = ["on_request"]

    config = {
      secret                    = "<base64 of at least 32 random bytes>"
      issuer                    = "https://auth.example.com"
      audience                  = "https://api.example.com"
      user_id_claim             = "sub"
      tenant_id_claim           = "tenant_id"
      public_paths              = "/health,/ready"
      token_type                = "at+jwt"
      clock_skew_leeway_seconds = "60"
      revoked_jti               = "id-1,id-2"
    }
  }
]
```

:::caution
Set `fail_policy = "closed"` for an auth device.
Under `"open"`, a device that fails to load or traps lets the request through unauthenticated instead of rejecting it.
:::

:::tip
Declaring `hooks = ["on_request"]` avoids paying for the five unused hook instantiations per request.
:::

## Config Keys

| Key                         | Required | Default | Description                                                                                                                                                                  |
|-----------------------------|----------|---------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `secret`                    | yes      |         | HS256 shared secret. Standard base64 (alphabet `+/`, padded) decoding to at least 32 bytes. Shorter secrets are rejected at load.                                            |
| `issuer`                    | yes      |         | Expected `iss` claim. A different or missing issuer is rejected.                                                                                                             |
| `audience`                  | yes      |         | Expected `aud` claim. Matches a string `aud` or an entry within an array `aud`.                                                                                              |
| `user_id_claim`             | no       | `sub`   | Claim mapped to `X-User-Id`. A token that lacks a scalar value for this claim is rejected.                                                                                   |
| `tenant_id_claim`           | no       |         | Claim mapped to `X-Tenant-Id`. Omit to skip tenant injection.                                                                                                                |
| `public_paths`              | no       |         | Comma-separated exact paths that bypass authentication.                                                                                                                      |
| `token_type`                | no       |         | Expected `typ` header. When set, a token whose type differs is rejected. The match is case-insensitive and the `application/` prefix is ignored. Omit to skip type checking. |
| `clock_skew_leeway_seconds` | no       | `0`     | Allowed clock skew in seconds, applied to both `exp` and `nbf`.                                                                                                              |
| `revoked_jti`               | no       |         | Comma-separated revoked token ids. When set, revocation is enforced and every token must carry a `jti` that is not in this list.                                             |

Generate a secret with:

```shell
openssl rand -base64 32
```

## Validation Rules

A request is rejected with `401` when any of the following holds:

- No `Authorization: Bearer <token>` header is present.
- The token is malformed, or its algorithm is not `HS256`.
- The signature does not verify against `secret`.
- `iss` does not match `issuer`, or `aud` does not include `audience`.
- `exp` is absent, or the token has expired. Expiry is mandatory.
- `nbf` is present and the token is not yet valid.
- The configured `user_id_claim` is missing or is not a scalar value.
- The resolved identity value is empty or contains control characters.
- `token_type` is configured and the token `typ` does not match.
- `revoked_jti` is configured and the token has no `jti`, or its `jti` is on the denylist.

The clock skew leeway widens both the `exp` and `nbf` bounds by the configured number of seconds.

## Identity Headers

On success the device rewrites identity headers so the upstream can trust them:

- `X-User-Id` is set from `user_id_claim`.
- `X-Tenant-Id` is set from `tenant_id_claim` when that claim is configured and present.
- Any client-supplied `X-User-Id` and `X-Tenant-Id` are removed first, unconditionally.
- The `Authorization` header is stripped so the raw token never reaches the upstream.

## Public Paths

Paths listed in `public_paths` bypass token validation.
The match is exact, so `/health` matches only `/health`.

Even on a bypass the device removes any client-supplied `X-User-Id` and `X-Tenant-Id`, so a request to a public path
cannot spoof identity to an upstream that trusts those headers.

## Revocation

Set `revoked_jti` to a comma-separated list of token ids to reject specific tokens without rotating the secret.
When the list is non-empty, revocation is enforced: the device rejects a token whose `jti` is on the list, and also
rejects any token that has no `jti`, so that every accepted token is revocable.
When `revoked_jti` is unset, `jti` is ignored.

Updating the list takes effect on the next configuration reload.
Because `exp` is mandatory and lifetimes should be short, an id can be dropped from the list once its token would have
expired, which keeps the list bounded.

:::note
This is a small denylist delivered through config, suited to a modest number of revoked ids.
It is not a scalable revocation service, and updates are not real time.
:::

## Error Responses

All rejections return a generic JSON body so the specific reason is not exposed to the client:

- `401` with `{"error":"unauthorized"}` for any authentication failure.
- `500` with `{"error":"internal error"}` for a device misconfiguration.

The specific reason is recorded in the Snakeway log stream.

## See Also

- [Authoring WASM Devices](../authoring-wasm-devices.md) for building your own device.
- [WASM Device Internals](../../internals/wasm-devices.md) for how the host applies device patches.
