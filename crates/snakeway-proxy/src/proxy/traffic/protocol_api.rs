//! Applies the resolved [`ProtocolMode`] to the upstream peer.
//!
//! The mode itself lives in `snakeway_engine::traffic` and is carried on the
//! request context.

use crate::proxy::TrafficProxy;
use pingora::BError;
use pingora::prelude::HttpPeer;
use snakeway_engine::ctx::RequestCtx;
use snakeway_engine::runtime::UpstreamRuntime;
use snakeway_engine::traffic::{ProtocolFacts, ProtocolMode};

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
