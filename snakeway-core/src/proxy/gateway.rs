use crate::ctx::{RequestCtx, ResponseCtx, WsCloseCtx, WsCtx};
use crate::device::core::pipeline::DevicePipeline;
use crate::device::core::result::DeviceResult;
use crate::proxy::gateway_ctx::GatewayCtx;
use crate::proxy::handlers::{AdminHandler, StaticFileHandler};
use crate::proxy::request_classification::{RequestKind, classify_request};
use crate::route::RouteKind;
use crate::server::ReloadHandle;
use crate::server::{RuntimeState, UpstreamRuntime};
use crate::traffic::{ServiceId, TrafficDirector, TrafficManager, UpstreamOutcome};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use http::{StatusCode, Version, header};
use pingora::prelude::*;
use pingora_http::{RequestHeader, ResponseHeader};
use std::sync::Arc;

/// Gateway is the core orchestration abstraction in Snakeway.
/// It wraps Pingora hooks and applies traffic decisions and device lifecycle hooks.
pub struct Gateway {
    gw_ctx: GatewayCtx,
    traffic_director: TrafficDirector,

    // Handlers
    admin_handler: AdminHandler,
    static_file_handler: StaticFileHandler,
}

impl Gateway {
    pub fn new(
        state: Arc<ArcSwap<RuntimeState>>,
        traffic_manager: Arc<TrafficManager>,
        reload: Arc<ReloadHandle>,
    ) -> Self {
        let admin_handler = AdminHandler::new(traffic_manager.clone(), reload);
        let gw_ctx = GatewayCtx::new(state, traffic_manager);
        Self {
            gw_ctx,
            traffic_director: TrafficDirector,
            admin_handler,
            static_file_handler: StaticFileHandler,
        }
    }
}

#[async_trait]
impl ProxyHttp for Gateway {
    type CTX = RequestCtx;

    fn new_ctx(&self) -> Self::CTX {
        RequestCtx::empty()
    }

    /// Select upstream and enforce protocol rules
    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let state = self.gw_ctx.state();

        let service_name = ctx
            .service
            .as_ref()
            .ok_or_else(|| Error::new(Custom("no service selected")))?;
        let service_id = ServiceId(service_name.clone());

        let upstream = self.select_upstream(ctx, &state, &service_id, service_name)?;

        let mut peer = HttpPeer::new(
            (upstream.host.as_str(), upstream.port),
            upstream.use_tls,
            upstream.sni.clone(),
        );

        // Enforce protocol rules for this upstream and request.
        self.enforce_protocol(&mut peer, ctx, upstream)?;

        // Set upstream authority for gRPC requests.
        let authority = format!("{}:{}", upstream.host, upstream.port);
        ctx.upstream_authority = Some(authority);

        // Record that this request was admitted by the circuit breaker.
        // The TrafficDirector already called `circuit_allows` for selection.
        ctx.cb_started = true;

        self.gw_ctx
            .traffic_manager
            .on_request_start(&service_id, &upstream.id);

        ctx.selected_upstream = Some((service_id, upstream.id));

        Ok(Box::new(peer))
    }

    /// ACCEPT → INSPECT → ROUTE → (RESPOND | PROXY)
    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        // The request ctx exists before now, but has no data.
        ctx.hydrate_from_session(session);

        let req = session.req_header();

        match classify_request(req) {
            RequestKind::Admin { path } => {
                // Admin endpoints
                // Note: These run on the main listener and currently have no authentication.
                // In the future, these may be moved to a separate internal listener or have auth applied.
                return self.admin_handler.handle(session, &path).await;
            }
            RequestKind::Normal => {}
        }

        let state = self.gw_ctx.state();

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
                if ctx.is_upgrade_req {
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

            RouteKind::Proxy {
                upstream,
                allow_websocket,
            } => {
                // If it is a websocket upgrade request, check if the upstream supports websockets.
                if ctx.is_upgrade_req && !allow_websocket {
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
        if ctx.is_grpc {
            let authority = ctx
                .upstream_authority()
                .ok_or_else(|| Error::new(Custom("missing upstream authority for gRPC")))?;

            upstream.insert_header(header::HOST, authority)?;
        }

        let state = self.gw_ctx.state();

        match DevicePipeline::run_before_proxy(state.devices.all(), ctx) {
            DeviceResult::Continue => {
                // Applies upstream intent derived from the request context.
                upstream.set_method(ctx.method.clone());
                upstream.set_uri(ctx.upstream_path().parse().unwrap());

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

            DeviceResult::Respond(_resp) => Err(Error::new(Custom("respond before proxy"))),

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
        let state = self.gw_ctx.state();

        match DevicePipeline::run_after_proxy(state.devices.all(), &mut resp_ctx) {
            DeviceResult::Continue => {}
            DeviceResult::Respond(_) => {}
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

            // Run WS-open hook.
            DevicePipeline::run_on_ws_open(self.gw_ctx.state().devices.all(), &WsCtx::default());
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
        if ctx.ws_opened || ctx.is_grpc {
            // Do not run on_response devices for WebSockets or gRPC.
            // For WebSockets and gRPC, this is not a real "response."
            // It is a protocol switch.
            return Ok(());
        }

        let mut resp_ctx = ResponseCtx::new(upstream.status, upstream.headers.clone(), Vec::new());
        let state = self.gw_ctx.state();
        match DevicePipeline::run_on_response(state.devices.all(), &mut resp_ctx) {
            DeviceResult::Continue => {}
            DeviceResult::Respond(_) => {}
            DeviceResult::Error(err) => {
                // Too late to change anything; logs and metrics only allowed here.
                tracing::warn!("device error on_response: {err}");
            }
        }

        upstream.set_status(resp_ctx.status)?;

        let status = upstream.status.as_u16();
        ctx.upstream_outcome = Some(if status >= 500 {
            UpstreamOutcome::HttpStatus(status)
        } else {
            UpstreamOutcome::Success
        });

        Ok(())
    }

    async fn logging(&self, _session: &mut Session, e: Option<&Error>, ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        if let Some((service_id, upstream_id)) = ctx.selected_upstream.as_ref() {
            // Transport-level failure wins over HTTP status.
            if e.is_some() {
                ctx.upstream_outcome = Some(UpstreamOutcome::TransportError);
            }

            let success = match ctx.upstream_outcome {
                Some(UpstreamOutcome::TransportError) => false,
                Some(UpstreamOutcome::HttpStatus(code)) => {
                    // Decide if this status code is a failure.
                    // By default, 5xx is failure.
                    let count_5xx = self
                        .gw_ctx
                        .traffic_manager
                        .circuit_params
                        .get(service_id)
                        .map(|p| p.count_http_5xx_as_failure)
                        .unwrap_or(true);

                    if count_5xx { code < 500 } else { true }
                }
                Some(UpstreamOutcome::Success) | None => true,
            };

            if success {
                self.gw_ctx
                    .traffic_manager
                    .report_success(service_id, upstream_id);
            } else {
                self.gw_ctx
                    .traffic_manager
                    .report_failure(service_id, upstream_id);
            }

            // Circuit breaker state update
            self.gw_ctx.traffic_manager.circuit_on_end(
                service_id,
                upstream_id,
                ctx.cb_started,
                success,
            );
        }

        // Always decrement active request counter
        if let Some((service_id, upstream_id)) = ctx.selected_upstream.take() {
            self.gw_ctx
                .traffic_manager
                .on_request_end(&service_id, &upstream_id);
        }

        // It may seem odd to put this in a "logging" hook, but it is the only way to do it.
        // Pingora guarantees the logging hook is called last, which is the best that can be
        // done in Pingora 0.6.0.
        if ctx.ws_opened {
            DevicePipeline::run_on_ws_close(
                self.gw_ctx.state().devices.all(),
                &WsCloseCtx::default(),
            );
        }
    }
}

impl Gateway {
    /// Select an upstream for the given request.
    fn select_upstream<'a>(
        &self,
        ctx: &RequestCtx,
        state: &'a RuntimeState,
        service_id: &ServiceId,
        service_name: &str,
    ) -> std::result::Result<&'a UpstreamRuntime, BError> {
        // Get a snapshot (cheap, lock-free)
        let snapshot = self.gw_ctx.traffic_manager.snapshot();

        // Ask the director for a decision.
        let decision = self
            .traffic_director
            .decide(ctx, &snapshot, service_id, &self.gw_ctx.traffic_manager)
            .map_err(|e| {
                tracing::error!(error = ?e, "traffic decision failed");
                Error::new(Custom("traffic decision failed"))
            })?;

        // Grab the service by name.
        let service = state
            .services
            .get(service_name)
            .ok_or_else(|| Error::new(Custom("unknown service")))?;

        // Get the upstream based on the decision from the Traffic Director.
        let upstream = service
            .upstreams
            .iter()
            .find(|u| u.id == decision.upstream_id)
            .ok_or_else(|| Error::new(Custom("selected upstream not found")))?;

        Ok(upstream)
    }

    /// Enforces protocol rules for the given upstream and request.
    ///
    /// PROTOCOL PRECEDENCE (highest to lowest):
    /// 1. WebSocket: HTTP/1.1 only
    /// 2. gRPC: HTTP/2 only (TLS required)
    /// 3. Default: Pingora defaults
    pub fn enforce_protocol(
        &self,
        peer: &mut HttpPeer,
        ctx: &RequestCtx,
        upstream: &UpstreamRuntime,
    ) -> Result<(), BError> {
        if ctx.is_upgrade_req {
            // WebSockets MUST be HTTP/1.1
            peer.options.set_http_version(1, 1);
        } else if ctx.is_grpc {
            if !upstream.use_tls {
                return Err(Error::new(Custom("gRPC upstream must use TLS and HTTP/2")));
            }
            peer.options.set_http_version(2, 2);
        }
        Ok(())
    }
}
