use async_trait::async_trait;
use pingora::prelude::*;
use pingora_http::{RequestHeader, ResponseHeader};

#[cfg(feature = "static_files")]
use tokio::io::AsyncReadExt;

use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::pipeline::DevicePipeline;
use crate::device::core::registry::DeviceRegistry;
use crate::device::core::result::DeviceResult;
use crate::route::{RouteEntry, RouteKind, Router};

pub struct SnakewayGateway {
    pub upstream_host: String,
    pub upstream_port: u16,
    pub use_tls: bool,
    pub sni: String,

    pub devices: DeviceRegistry,
    pub router: Router,
}

#[async_trait]
impl ProxyHttp for SnakewayGateway {
    type CTX = RequestCtx;

    fn new_ctx(&self) -> Self::CTX {
        // Placeholder; real initialization happens in request_filter
        RequestCtx::new(
            http::Method::GET,
            "/".parse().unwrap(),
            http::HeaderMap::new(),
            Vec::new(),
        )
    }

    /// Simple upstream peer selection
    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let addr = (self.upstream_host.as_str(), self.upstream_port);
        let peer = HttpPeer::new(addr, self.use_tls, self.sni.clone());
        Ok(Box::new(peer))
    }

    /// Snakeway `on_request` --> Pingora `request_filter`
    ///
    /// Intent:
    /// ACCEPT --> INSPECT --> DECIDE --> (RESPOND | PROXY)
    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        let req = session.req_header();

        *ctx = RequestCtx::new(
            req.method.clone(),
            req.uri.clone(),
            req.headers.clone(),
            Vec::new(),
        );

        // Run on_request devices first (applies to both static and upstream requests).
        match DevicePipeline::run_on_request(self.devices.all(), ctx) {
            DeviceResult::Continue => {}

            DeviceResult::Respond(resp) => {
                session.respond_error(resp.status.as_u16()).await?;
                return Ok(true);
            }

            DeviceResult::Error(err) => {
                tracing::error!("device error in on_request: {err}");
                session.respond_error(500).await?;
                return Ok(true);
            }
        }

        // Make a decision about the route.
        let route = match self.router.match_route(&ctx.route_path) {
            Ok(r) => r,
            Err(err) => {
                tracing::warn!("no route matched: {err}");
                session.respond_error(404).await?;
                return Ok(true);
            }
        };

        match &route.kind {
            RouteKind::Static { .. } => {
                respond_with_static(session, ctx, route, &self.devices).await
            }

            RouteKind::Proxy { .. } => {
                // Proceed to upstream routing...
                Ok(false)
            }
        }
    }

    /// Snakeway `before_proxy` --> Pingora `upstream_request_filter`
    ///
    /// Intent:
    /// MUTATE OR ABORT UPSTREAM
    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        match DevicePipeline::run_before_proxy(self.devices.all(), ctx) {
            DeviceResult::Continue => {
                // Applies upstream intent derived from the request context
                upstream.set_method(ctx.method.clone());
                let path = ctx.upstream_path();
                upstream.set_uri(path.parse().unwrap());

                Ok(())
            }

            DeviceResult::Respond(_resp) => {
                // We cannot write a response here; aborting forces Pingora
                // to unwind and prevents upstream dispatch.
                tracing::info!("request responded before proxy");
                Err(Error::new(Custom("respond before proxy")))
            }

            DeviceResult::Error(err) => {
                tracing::error!("device error before_proxy: {err}");
                Err(Error::new(Custom("device error before proxy")))
            }
        }
    }

    /// Snakeway `after_proxy` --> Pingora `upstream_response_filter`
    ///
    /// Intent:
    /// MUTATE RESPONSE HEADERS / STATUS
    fn upstream_response_filter(
        &self,
        _session: &mut Session,
        upstream: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        let mut resp_ctx = ResponseCtx::new(upstream.status, upstream.headers.clone(), Vec::new());

        match DevicePipeline::run_after_proxy(self.devices.all(), &mut resp_ctx) {
            DeviceResult::Continue => {}

            DeviceResult::Respond(_resp) => {
                // Legal here: treat as override of response fields
                tracing::debug!("response overridden in after_proxy");
            }

            DeviceResult::Error(err) => {
                // Response is already committed; we only record and observe
                tracing::warn!("device error after_proxy: {err}");
            }
        }

        upstream.set_status(resp_ctx.status)?;
        Ok(())
    }

    /// Snakeway `on_response` --> Pingora `response_filter`
    ///
    /// Intent:
    /// FINAL OBSERVATION / METRICS / LOGGING
    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        let mut resp_ctx = ResponseCtx::new(upstream.status, upstream.headers.clone(), Vec::new());

        match DevicePipeline::run_on_response(self.devices.all(), &mut resp_ctx) {
            DeviceResult::Continue => {}

            DeviceResult::Respond(_resp) => {
                tracing::debug!("response overridden in on_response");
            }

            DeviceResult::Error(err) => {
                // Too late to change anything; log + metric only
                tracing::warn!("device error on_response: {err}");
            }
        }

        upstream.set_status(resp_ctx.status)?;
        Ok(())
    }
}

#[cfg(not(feature = "static_files"))]
pub async fn respond_with_static(
    _session: &mut Session,
    _ctx: &RequestCtx,
    _route: &RouteEntry,
    _devices: &DeviceRegistry,
) -> Result<bool> {
    Err(Error::new(Custom("static files disabled")))
}

#[cfg(feature = "static_files")]
pub async fn respond_with_static(
    session: &mut Session,
    ctx: &RequestCtx,
    route: &RouteEntry,
    devices: &DeviceRegistry,
) -> Result<bool> {
    // Extract conditional headers for cache validation and content negotiation.
    let conditional = crate::static_files::ConditionalHeaders {
        if_none_match: ctx
            .headers
            .get(http::header::IF_NONE_MATCH)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        if_modified_since: ctx
            .headers
            .get(http::header::IF_MODIFIED_SINCE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        accept_encoding: ctx
            .headers
            .get(http::header::ACCEPT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        range: ctx
            .headers
            .get(http::header::RANGE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
    };

    let static_resp =
        crate::static_files::handle_static_request(&route.kind, &ctx.route_path, &conditional)
            .await;

    // Build response header
    let mut resp = ResponseHeader::build(static_resp.status, None)?;

    // Copy headers
    for (name, value) in static_resp.headers.iter() {
        resp.insert_header(name, value)?;
    }

    // Write headers (not end-of-stream yet)
    session.write_response_header(Box::new(resp), false).await?;

    let is_head = ctx.method == http::Method::HEAD;
    if is_head {
        // SHort-circuit the body write step for HEAD requests.
        session.write_response_body(None, true).await?;
    } else {
        // Write body and end the stream.
        match static_resp.body {
            crate::static_files::StaticBody::Empty => {
                session.write_response_body(None, true).await?;
            }

            crate::static_files::StaticBody::Bytes(bytes) => {
                session.write_response_body(Some(bytes), true).await?;
            }

            crate::static_files::StaticBody::File(mut file) => {
                use bytes::{Bytes, BytesMut};
                use tokio::io::AsyncReadExt;

                const CHUNK_SIZE: usize = 32 * 1024;

                // Allocate once per request.
                let mut buf = BytesMut::with_capacity(CHUNK_SIZE);

                loop {
                    // Ensure we have space to read into.
                    buf.resize(CHUNK_SIZE, 0);

                    let n = file
                        .read(&mut buf[..])
                        .await
                        .map_err(|_| Error::new(Custom("static file read error")))?;

                    if n == 0 {
                        break;
                    }

                    // Shrink to actual read size.
                    buf.truncate(n);

                    // Split off the filled bytes and freeze them.
                    let chunk: Bytes = buf.split().freeze();

                    session.write_response_body(Some(chunk), false).await?;
                }

                // End-of-stream.
                session.write_response_body(None, true).await?;
            }

            crate::static_files::StaticBody::RangedFile {
                mut file,
                mut remaining,
            } => {
                const CHUNK_SIZE: usize = 32 * 1024;
                let mut buf = bytes::BytesMut::with_capacity(CHUNK_SIZE);

                while remaining > 0 {
                    let to_read = std::cmp::min(CHUNK_SIZE as u64, remaining) as usize;

                    buf.resize(to_read, 0);

                    let n = file
                        .read(&mut buf[..])
                        .await
                        .map_err(|_| Error::new(Custom("static file read error")))?;

                    if n == 0 {
                        break;
                    }

                    remaining -= n as u64;
                    buf.truncate(n);

                    session
                        .write_response_body(Some(buf.split().freeze()), false)
                        .await?;
                }

                session.write_response_body(None, true).await?;
            }
        }
    }

    // Run on_response devices
    let mut resp_ctx = ResponseCtx::new(static_resp.status, static_resp.headers, Vec::new());

    match DevicePipeline::run_on_response(devices.all(), &mut resp_ctx) {
        DeviceResult::Continue => {}
        DeviceResult::Respond(_) => {}
        DeviceResult::Error(err) => {
            tracing::warn!("device error on_response (static): {err}");
        }
    }

    Ok(true)
}
