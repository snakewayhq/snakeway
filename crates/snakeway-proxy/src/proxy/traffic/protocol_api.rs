//! Explicit HTTP version negotiation.
//!
//! `ProtocolMode` names the end-to-end wire outcome that the current code
//! re-derives at several hooks from `upstream.version` and `is_upgrade_req()`.
//! It is resolved once, from the request and the selected upstream, by
//! [`ProtocolMode::resolve`].
//!
//! Upgrade negotiation and the upstream `Host` policy are deliberately not part
//! of this type. An active upgrade only constrains the version to HTTP/1.1, and
//! the `Host` value is a separate policy as it depends on the protocol version.

use crate::proxy::TrafficProxy;
use pingora::BError;
use pingora::prelude::HttpPeer;
use snakeway_engine::ctx::RequestCtx;
use snakeway_engine::runtime::UpstreamRuntime;

impl TrafficProxy {
    /// Enforces protocol rules for the given upstream and request.
    ///
    /// PROTOCOL PRECEDENCE (highest to lowest):
    /// 1. WebSocket: HTTP/1.1 only
    /// 2. HTTP/2 + TLS upstream: end-to-end h2 (gRPC, h2-to-h2)
    /// 3. HTTP/2 + plaintext upstream: h2-to-h1 (Pingora translates)
    /// 4. Default: Pingora defaults
    pub(crate) fn enforce_protocol(
        &self,
        peer: &mut HttpPeer,
        ctx: &RequestCtx,
        upstream: &UpstreamRuntime,
    ) -> pingora::Result<ProtocolMode, BError> {
        let is_upgrade = ctx.is_upgrade_req();
        let mode = ProtocolMode::resolve(ProtocolFacts {
            downstream_http2: ctx.is_http2(),
            upstream_tls: upstream.use_tls(),
            is_upgrade,
        });
        match mode {
            ProtocolMode::Http2EndToEnd => peer.options.set_http_version(2, 2),
            // An upgrade is forced to HTTP/1.1 so ALPN cannot negotiate h2 on a
            // TLS upstream. A non-upgrade HTTP/1.1 request keeps Pingora's default.
            ProtocolMode::Http1 if is_upgrade => peer.options.set_http_version(1, 1),
            ProtocolMode::Http1 => {}
        }
        Ok(mode)
    }
}

/// The negotiated wire protocol between the proxy and the upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProtocolMode {
    /// HTTP/1.1 to the upstream. Covers an HTTP/1.x client, and an HTTP/2
    /// client whose upstream is plaintext (Pingora translates h2 to h1).
    Http1,
    /// End-to-end HTTP/2: an HTTP/2 client proxied to a TLS upstream.
    Http2EndToEnd,
}

/// The facts that determine the version outcome.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProtocolFacts {
    /// The downstream request is HTTP/2.
    pub downstream_http2: bool,
    /// The selected upstream uses TLS.
    pub upstream_tls: bool,
    /// The request is a protocol upgrade (for example a WebSocket handshake).
    pub is_upgrade: bool,
}

impl ProtocolMode {
    /// Resolves the wire protocol from the request and the selected upstream.
    ///
    /// An upgrade forces HTTP/1.1 (the mechanism is HTTP/1.1 only). Otherwise,
    /// end-to-end HTTP/2 requires both an HTTP/2 client and a TLS upstream.
    /// Every other combination is HTTP/1.1.
    pub(crate) fn resolve(inputs: ProtocolFacts) -> Self {
        if inputs.is_upgrade {
            Self::Http1
        } else if inputs.downstream_http2 && inputs.upstream_tls {
            Self::Http2EndToEnd
        } else {
            Self::Http1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end HTTP/2 requires both an HTTP/2 client and a TLS upstream.
    #[test]
    fn should_require_tls_upstream_for_end_to_end_h2() {
        // Arrange
        let inputs = ProtocolFacts {
            downstream_http2: true,
            upstream_tls: true,
            is_upgrade: false,
        };

        // Act
        let mode = ProtocolMode::resolve(inputs);

        // Assert
        assert_eq!(mode, ProtocolMode::Http2EndToEnd);
    }

    /// An HTTP/2 client to a plaintext upstream is HTTP/1.1 (Pingora translates).
    #[test]
    fn should_resolve_to_plaintext_http1_upstream_for_http2_client() {
        // Arrange
        let inputs = ProtocolFacts {
            downstream_http2: true,
            upstream_tls: false,
            is_upgrade: false,
        };

        // Act
        let mode = ProtocolMode::resolve(inputs);

        // Assert
        assert_eq!(mode, ProtocolMode::Http1);
    }

    /// An active upgrade forces HTTP/1.1 even when the h2-plus-TLS conditions hold.
    #[test]
    fn should_force_http1_over_end_to_end_h2_when_upgrade() {
        // Arrange
        let inputs = ProtocolFacts {
            downstream_http2: true,
            upstream_tls: true,
            is_upgrade: true,
        };

        // Act
        let mode = ProtocolMode::resolve(inputs);

        // Assert
        assert_eq!(mode, ProtocolMode::Http1);
    }

    /// The full three-input truth table, so no combination is left undefined.
    #[test]
    fn should_resolve_all_states() {
        // Arrange: (is_upgrade, downstream_http2, upstream_tls) -> expected mode.
        let cases = [
            (false, false, false, ProtocolMode::Http1),
            (false, false, true, ProtocolMode::Http1),
            (false, true, false, ProtocolMode::Http1),
            (false, true, true, ProtocolMode::Http2EndToEnd),
            (true, false, false, ProtocolMode::Http1),
            (true, false, true, ProtocolMode::Http1),
            (true, true, false, ProtocolMode::Http1),
            (true, true, true, ProtocolMode::Http1),
        ];

        for (is_upgrade, downstream_http2, upstream_tls, expected) in cases {
            // Act
            let mode = ProtocolMode::resolve(ProtocolFacts {
                downstream_http2,
                upstream_tls,
                is_upgrade,
            });

            // Assert
            assert_eq!(
                mode, expected,
                "is_upgrade={is_upgrade} downstream_http2={downstream_http2} upstream_tls={upstream_tls}"
            );
        }
    }
}
