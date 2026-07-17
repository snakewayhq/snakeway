---
title: Protocol Negotiation
---

HTTP protocol negotiation covers two independent concerns:

1. Version negotiation: which HTTP version Snakeway speaks to the upstream.
2. Upgrade negotiation: whether the connection is turned into a tunnel, as for a WebSocket.

They are separate workflows/machines.

1. Version negotiation is a classification made once, when the upstream is selected.
2. Upgrade negotiation is a lifecycle that unfolds across several proxy hooks and is driven by the upstream response.

Version negotiation is a pure classification.
Three inputs map to one mode.
It is computed at upstream selection and never revised.

Upgrade negotiation is stateful.
It spans multiple hooks, is driven by the upstream `101` response, and has terminal rejection at two layers: the gateway
and the upstream itself.
An upgrade constrains version negotiation to HTTP/1.1.

## Version negotiation

The version is resolved once, in the `upstream_peer` hook, from three facts: whether the downstream request is HTTP/2,
whether the selected upstream uses TLS, and whether the request is an upgrade.
The result is a `ProtocolMode` (`Http1` or `Http2EndToEnd`) stored on the request context and read by the later hooks,
so the outcome is decided in a single place instead of being derived again at each hook.

HTTP/2 is offered to clients only over TLS.
There is no cleartext HTTP/2 (h2c) listener.

The table maps request shape to outcome.
The `Host source` column is the value Snakeway sends as the upstream `Host` (or the HTTP/2 `:authority`).
An upgrade resolves to `Http1`, with the handshake handled by the upgrade machine described below.

| Downstream | Initiation                  | Upstream TLS | Resolved mode                                            | Host source                                                                                      |
|------------|-----------------------------|--------------|----------------------------------------------------------|--------------------------------------------------------------------------------------------------|
| HTTP/2     | none                        | TLS          | `Http2EndToEnd`                                          | upstream authority (overrides client)                                                            |
| HTTP/2     | none                        | plaintext    | `Http1`                                                  | client `:authority` via downstream authority, or the client `Host` if the h2 request carried one |
| HTTP/1.1   | none                        | TLS          | `Http1`                                                  | client `Host` header (passed through)                                                            |
| HTTP/1.1   | none                        | plaintext    | `Http1`                                                  | client `Host` header (passed through)                                                            |
| HTTP/1.1   | Upgrade                     | any          | `Http1`                                                  | client `Host` header (passed through)                                                            |
| HTTP/1.0   | none, with `Host`           | any          | `Http1` (upgraded to h1.1 upstream)                      | client `Host` header (passed through)                                                            |
| HTTP/1.0   | none, without `Host`        | any          | rejected with 400 (no authority to forward)              | n/a                                                                                              |
| HTTP/1.1   | none, without `Host`        | any          | rejected with 400 (no authority to forward)              | n/a                                                                                              |
| HTTP/2     | Upgrade header              | any          | rejected at the h2 codec as malformed (never reaches us) | n/a                                                                                              |
| HTTP/2     | Extended CONNECT (RFC 8441) | any          | not supported: Pingora resets the stream                 | n/a                                                                                              |

## Upgrade negotiation

The only supported upgrade mechanism is the HTTP/1.1 `Upgrade` handshake (a WebSocket).
It forces HTTP/1.1 to the upstream regardless of the version negotiation, because the mechanism does not exist in
HTTP/2.
WebSocket over HTTP/2 (RFC 8441 Extended CONNECT) is not supported.
Snakeway does not advertise `SETTINGS_ENABLE_CONNECT_PROTOCOL`, and such a request is reset.
Foundationally, this is because Pingora does not support websockets over HTTP/2.

An upgrade progresses through these states:

| From       | Event                                                         | To                           |
|------------|---------------------------------------------------------------|------------------------------|
| NotUpgrade | `request_filter` sees a valid `Upgrade`                       | Requested                    |
| Requested  | the route allows WebSockets and a connection slot is acquired | Admitted                     |
| Requested  | the route forbids WebSockets, or the pool is full             | GatewayRejected (426 or 503) |
| Admitted   | `upstream_request_filter` forces h1 and sets upgrade headers  | Negotiated                   |
| Negotiated | upstream returns `101`                                        | Switched                     |
| Negotiated | upstream returns a status other than `101`                    | UpstreamRejected (forwarded) |
| Negotiated | the upstream connection fails or aborts before `101`          | Failed                       |
| Switched   | either side closes                                            | Closed                       |

Reaching `Switched` runs the WebSocket open hook and suppresses the normal response lifecycle.
`Closed` runs the WebSocket close hook.
A rejected or failed handshake runs neither.

There are two rejection layers.

1. GatewayRejected happens in `request_filter`, before any upstream is contacted.
2. UpstreamRejected happens when the upstream answers the handshake with a status other than `101`.

Known limitation: the `Negotiated` state has no timeout.
An upstream that connects but never sends `101` will hang, because Pingora's single read timeout cannot bound the
handshake without also tearing down an idle established tunnel.
