//! Request intake for the `on_request` phase: root span setup, the
//! `on_request` device run, and route dispatch.

use crate::proxy::TrafficProxy;
use crate::proxy::traffic::DeclaredContentLength;
use http::{StatusCode, header};
use pingora::prelude::Session;
use pingora::{Custom, Error};
use snakeway_engine::ctx::RequestCtx;
use snakeway_engine::device::builtin::request_filter::ClientBodyTimeout;
use snakeway_engine::device::core::{DevicePipeline, DeviceResult};
use snakeway_engine::route::RouteRuntime;
use snakeway_engine::runtime::RuntimeState;
use snakeway_observability::HeaderExtractor;
use tracing_opentelemetry::OpenTelemetrySpanExt;

impl TrafficProxy {
    /// Opens the per-request root span and stores it on the context.
    pub(in crate::proxy) fn open_request_span(&self, session: &Session, ctx: &mut RequestCtx) {
        // Extract W3C Trace Context from downstream request headers.
        // When no traceparent header is present, the context is empty and
        // set_parent below becomes a no-op (the span stays a root).
        let parent_cx = opentelemetry::global::get_text_map_propagator(|prop| {
            prop.extract(&HeaderExtractor(&session.req_header().headers))
        });

        let request_id = ctx.request_id().unwrap_or_else(|| "unknown".into());

        let span = tracing::info_span!(
            "request",
            http.method = %ctx.method_str(),
            http.host = %ctx.effective_host(),
            http.path = %ctx.canonical_path(),
            client.ip = %ctx.peer_ip,
            request.id = %request_id,
            listener = %self.listener,
            route = tracing::field::Empty,
        );

        // Link the request span to the extracted upstream trace context.
        let _ = span.set_parent(parent_cx);

        ctx.request_span = Some(span);
    }

    /// Runs the `on_request` device phase and applies its session effects.
    ///
    /// Returns true when a device already responded to the client.
    pub(in crate::proxy) async fn run_on_request_devices(
        &self,
        session: &mut Session,
        ctx: &mut RequestCtx,
        state: &RuntimeState,
    ) -> pingora::Result<bool> {
        match DevicePipeline::run_on_request(state.devices.all(), ctx) {
            DeviceResult::Continue => {}

            DeviceResult::Respond(resp) => {
                resp.write_to_session(&mut session.downstream_session)
                    .await?;
                return Ok(true);
            }

            DeviceResult::Error(err) => {
                tracing::error!("device error in on_request: {err}");
                session.respond_error(500).await?;
                return Ok(true);
            }
        }

        // Apply client body timeout if the request filter device configured one.
        if let Some(timeout) = ctx.extensions.get::<ClientBodyTimeout>() {
            session.downstream_session.set_read_timeout(Some(timeout.0));
        }

        // Store declared Content-Length for underflow detection in request_body_filter.
        if let Some(cl) = session
            .req_header()
            .headers
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<u64>().ok())
        {
            ctx.extensions.insert(DeclaredContentLength(cl));
        }

        Ok(false)
    }

    /// Matches the route for the request and dispatches it.
    ///
    /// Returns Ok(true) when the request was fully handled here (static file,
    /// rejection, or pool exhaustion), and Ok(false) when it proceeds upstream.
    pub(in crate::proxy) async fn dispatch_route(
        &self,
        session: &mut Session,
        ctx: &mut RequestCtx,
        state: &RuntimeState,
    ) -> pingora::Result<bool> {
        let router = state
            .routers
            .get(self.listener.as_ref())
            .ok_or_else(|| Error::new(Custom("no router for listener")))?;

        let route = match router.match_route(ctx.effective_host(), ctx.canonical_path()) {
            Ok(r) => r,
            Err(err) => {
                tracing::warn!("no route matched: {err}");
                session.respond_error(404).await?;
                return Ok(true);
            }
        };

        match &route.kind {
            RouteRuntime::Static { id, .. } => {
                ctx.route_id = Some(id.clone());
                if ctx.is_upgrade_req() {
                    // Reject websocket upgrade requests for static files.
                    session
                        .respond_error(StatusCode::BAD_REQUEST.as_u16())
                        .await?;
                    return Ok(true);
                }
                self.static_file_handler
                    .handle(session, ctx, route, &state.devices)
                    .await
            }

            RouteRuntime::Service {
                id,
                upstream,
                allow_websocket,
                ws_max_connections,
                ..
            } => {
                ctx.route_id = Some(id.clone());

                // If it is a websocket upgrade request, check if the upstream supports websockets.
                if ctx.is_upgrade_req() {
                    if !allow_websocket {
                        session
                            .respond_error(StatusCode::UPGRADE_REQUIRED.as_u16())
                            .await?;
                        return Ok(true);
                    }

                    // Acquire a connection slot for ws guard.
                    // A full pool is a 503 Service Unavailable (not a 500 Internal Server Error).
                    let Some(guard) = self
                        .proxy_ctx
                        .connection_manager
                        .try_acquire(id, ws_max_connections.to_owned())
                    else {
                        session
                            .respond_error(StatusCode::SERVICE_UNAVAILABLE.as_u16())
                            .await?;
                        return Ok(true);
                    };

                    ctx.ws_guard = Some(guard);
                }

                ctx.service = Some(upstream.clone());
                Ok(false)
            }
        }
    }
}
