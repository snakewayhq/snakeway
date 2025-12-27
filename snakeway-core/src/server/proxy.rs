use crate::ctx::{RequestCtx, ResponseCtx, WsCloseCtx, WsCtx};
use crate::device::core::pipeline::DevicePipeline;
use crate::device::core::registry::DeviceRegistry;
use crate::device::core::result::DeviceResult;
use crate::route::{RouteEntry, RouteKind};
use crate::server::runtime::RuntimeState;
use arc_swap::ArcSwap;
use async_trait::async_trait;
use http::{StatusCode, Version, header};
use pingora::prelude::*;
use pingora_http::{RequestHeader, ResponseHeader};
use std::net::Ipv4Addr;
use std::sync::Arc;
#[cfg(feature = "static_files")]
use tokio::io::AsyncReadExt;

pub struct SnakewayGateway {
    // Runtime state
    pub state: Arc<ArcSwap<RuntimeState>>,
}

#[async_trait]
impl ProxyHttp for SnakewayGateway {
    type CTX = RequestCtx;

    fn new_ctx(&self) -> Self::CTX {
        // Placeholder; real initialization happens in request_filter
        RequestCtx::new(
            None,
            http::Method::GET,
            "/".parse().unwrap(),
            http::HeaderMap::new(),
            Ipv4Addr::UNSPECIFIED.into(),
            false,
            Vec::new(),
        )
    }

    /// Simple upstream peer selection
    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let service_name = ctx
            .service
            .as_ref()
            .ok_or_else(|| Error::new(Custom("no service selected")))?;

        let state = self.state.load();

        let service = state
            .services
            .get(service_name)
            .ok_or_else(|| Error::new(Custom("unknown service")))?;

        let upstream = service
            .select_upstream()
            .ok_or_else(|| Error::new(Custom("no upstreams")))?;

        let peer = HttpPeer::new(
            (upstream.host.as_str(), upstream.port),
            upstream.use_tls,
            upstream.sni.clone(),
        );

        Ok(Box::new(peer))
    }

    /// Snakeway `on_request` --> Pingora `request_filter`
    ///
    /// Intent:
    /// ACCEPT --> INSPECT --> DECIDE --> (RESPOND | PROXY)
    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        let req = session.req_header();
        let is_upgrade_req = session.is_upgrade_req();

        *ctx = RequestCtx::new(
            None,
            req.method.clone(),
            req.uri.clone(),
            req.headers.clone(),
            ctx.peer_ip,
            is_upgrade_req,
            Vec::new(),
        );

        let state = self.state.load();

        // Run on_request devices first (applies to both static and upstream requests).
        match DevicePipeline::run_on_request(state.devices.all(), ctx) {
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
        let route = match state.router.match_route(&ctx.route_path) {
            Ok(r) => r,
            Err(err) => {
                tracing::warn!("no route matched: {err}");
                session.respond_error(404).await?;
                return Ok(true);
            }
        };

        match &route.kind {
            RouteKind::Static { .. } => {
                if is_upgrade_req {
                    // Reject websocket upgrade requests for static files.
                    session
                        .respond_error(StatusCode::BAD_REQUEST.as_u16())
                        .await?;
                    return Ok(true);
                }
                respond_with_static(session, ctx, route, &state.devices).await
            }

            RouteKind::Proxy {
                upstream,
                allow_websocket,
            } => {
                // If it is a websocket upgrade request, check if the upstream supports websockets.
                if is_upgrade_req && !allow_websocket {
                    session
                        .respond_error(StatusCode::UPGRADE_REQUIRED.as_u16())
                        .await?;
                    return Ok(true);
                }

                ctx.service = Some(upstream.clone());
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
        let state = self.state.load();

        match DevicePipeline::run_before_proxy(state.devices.all(), ctx) {
            DeviceResult::Continue => {
                // Applies upstream intent derived from the request context.
                upstream.set_method(ctx.method.clone());
                let path = ctx.upstream_path();
                upstream.set_uri(path.parse().unwrap());

                if ctx.is_upgrade_req {
                    // Upgrade is an HTTP/1.1 mechanism (HTTP/2 forbids it)
                    upstream.set_version(Version::HTTP_11);

                    // The headers are explicitly set - upstreams can be picky if they aren't there.
                    // Note that if the client already set these. they will be replaced.
                    upstream.insert_header(header::UPGRADE, "websocket")?;
                    upstream.insert_header(header::CONNECTION, "Upgrade")?;
                }

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
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        let mut resp_ctx = ResponseCtx::new(upstream.status, upstream.headers.clone(), Vec::new());
        let state = self.state.load();
        match DevicePipeline::run_after_proxy(state.devices.all(), &mut resp_ctx) {
            DeviceResult::Continue => {}

            DeviceResult::Respond(_resp) => {
                // Legal here: treat as override of response fields.
                tracing::debug!("response overridden in after_proxy");
            }

            DeviceResult::Error(err) => {
                // Response is already committed; we only record and observe.
                tracing::warn!("device error after_proxy: {err}");
            }
        }

        upstream.set_status(resp_ctx.status)?;

        if ctx.is_upgrade_req && upstream.status == StatusCode::SWITCHING_PROTOCOLS {
            // WS upgrade completed.
            // After this point, HTTP response lifecycle hooks (on_response)
            // must NOT run for this request.
            ctx.ws_opened = true;

            // Run WS-open hook
            let ws_ctx = WsCtx::default();
            DevicePipeline::run_on_ws_open(self.state.load().devices.all(), &ws_ctx);

            return Ok(());
        }

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
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        if ctx.ws_opened {
            // Do not run on_response devices for WebSockets.
            // For WebSockets, this is not a real "response."
            // It is a protocol switch.
            return Ok(());
        }

        let mut resp_ctx = ResponseCtx::new(upstream.status, upstream.headers.clone(), Vec::new());
        let state = self.state.load();
        match DevicePipeline::run_on_response(state.devices.all(), &mut resp_ctx) {
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
    async fn logging(&self, _session: &mut Session, _e: Option<&Error>, ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        // It may seem odd to put this in a "logging" hook, but it is the only way to do it.
        // Pingora guarantees the logging hook is called last, which is the best that can be
        // done in Pingora 0.6.0.
        if ctx.ws_opened {
            // Call device on_ws_close hook for WebSockets.
            let ws_close_ctx = WsCloseCtx::default();
            DevicePipeline::run_on_ws_close(self.state.load().devices.all(), &ws_close_ctx);
        }
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
