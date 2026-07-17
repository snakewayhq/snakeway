---
title: Protocol Negotiation
---

HTTP protocol negotiation is concerned with two areas:

1. Version negotiation
2. Upgrade negotiation

## Protocol Lifecycle State Transitions

| Downstream | Initiation                  | Upstream TLS | Resolved mode                                                                             | Host source                                                                               |
|------------|-----------------------------|--------------|-------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------|
| HTTP/2     | none                        | TLS          | Http2EndToEnd { authority }                                                               | upstream authority (overrides client)                                                     |
| HTTP/2     | none                        | plaintext    | Http1 { host_source }                                                                     | client `:authority` via downstream authority; client `Host` if the h2 request carried one |
| HTTP/1.1   | none                        | TLS          | Http1 { host_source }                                                                     | client `Host` header (pass-through)                                                       |
| HTTP/1.1   | none                        | plaintext    | Http1 { host_source }                                                                     | client `Host` header (pass-through)                                                       |
| HTTP/1.1   | Upgrade                     | TLS          | WebSocket                                                                                 | client `Host` header (pass-through)                                                       |
| HTTP/1.1   | Upgrade                     | plaintext    | WebSocket                                                                                 | client `Host` header (pass-through)                                                       |
| HTTP/2     | Upgrade header              | any          | rejected at the h2 codec as malformed (never reaches Snakeway's HttpProxy implementation) | n/a                                                                                       |
| HTTP/2     | Extended CONNECT (RFC 8441) | any          | not supported: Pingora resets the stream (protocol error)                                 | n/a                                                                                       |
| HTTP/1.0   | none, with `Host`           | any          | Http1 (upgraded to h1.1 upstream)                                                         | client `Host` header (pass-through)                                                       |
| HTTP/1.0   | none, without `Host`        | any          | rejected with 400 (client error)                                                          | none: no authority to forward                                                             |
