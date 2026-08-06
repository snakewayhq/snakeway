//! Shapes the upstream request from the device-visible request context.

use crate::proxy::traffic::headers::write_back_request_headers;
use http::{Version, header};
use pingora::http::RequestHeader;
use pingora::{Custom, Error};
use snakeway_engine::ctx::RequestCtx;
use snakeway_engine::traffic::ProtocolMode;
use snakeway_observability::RequestHeaderInjector;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Applies the request context's upstream intent to the outgoing request.
///
/// Covers the method and URI, the header writeback, the upstream Host policy
/// for the resolved protocol mode, upgrade forcing to HTTP/1.1, and W3C Trace
/// Context injection.
pub(in crate::proxy) fn apply_upstream_intent(
    upstream: &mut RequestHeader,
    ctx: &RequestCtx,
) -> pingora::Result<()> {
    upstream.set_method(ctx.method().to_owned());
    // Using try_from(String) is intentional.
    // It hands the buffer to the Uri without a copy.
    upstream.set_uri(
        http::Uri::try_from(ctx.upstream_uri())
            .map_err(|_| Error::new(Custom("invalid upstream uri")))?,
    );

    // Device header ops live on `ctx`, so the upstream request
    // headers are rebuilt from it.
    write_back_request_headers(upstream, ctx.headers())?;

    // The upstream Host follows the resolved protocol mode.
    // For end-to-end HTTP/2 it comes from the upstream authority
    // set in upstream_peer and overrides any client Host.
    let protocol_mode = ctx
        .protocol_mode
        .ok_or_else(|| Error::new(Custom("protocol mode not resolved")))?;

    match protocol_mode {
        ProtocolMode::Http2EndToEnd => {
            let authority = ctx
                .upstream_authority()
                .ok_or_else(|| Error::new(Custom("missing upstream authority for h2")))?;
            upstream.insert_header(header::HOST, authority)?;
        }
        ProtocolMode::Http1 => {
            if !upstream.headers.contains_key(header::HOST) {
                // An HTTP/2 downstream request carries its authority in
                // the `:authority` pseudo-header, which never appears in
                // the header map rebuilt above.
                // HTTP/1.1 requires Host (RFC 9112 §3.2), so derive it
                // from the request authority.
                let authority = ctx
                    .downstream_authority()
                    .ok_or_else(|| Error::new(Custom("missing authority for h1 upstream Host")))?;
                upstream.insert_header(header::HOST, authority)?;
            }
        }
    }

    if ctx.is_upgrade_req() {
        // Upgrade is an HTTP/1.1 mechanism (HTTP/2 forbids it)
        upstream.set_version(Version::HTTP_11);

        // The headers are explicitly set - upstreams can be picky if they aren't there.
        // Note that if the client already set these. they will be replaced.
        upstream.insert_header(header::UPGRADE, "websocket")?;
        upstream.insert_header(header::CONNECTION, "Upgrade")?;
    }

    // Inject W3C Trace Context into upstream request headers so
    // the upstream service can continue the distributed trace.
    opentelemetry::global::get_text_map_propagator(|prop| {
        prop.inject_context(
            &tracing::Span::current().context(),
            &mut RequestHeaderInjector(upstream),
        );
    });

    Ok(())
}
