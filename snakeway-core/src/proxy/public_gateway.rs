use crate::ctx::{RequestCtx, ResponseCtx, WsCloseCtx, WsCtx};
use crate::device::core::pipeline::DevicePipeline;
use crate::device::core::result::DeviceResult;
use crate::proxy::error_classification::classify_pingora_error;
use crate::proxy::gateway_ctx::GatewayCtx;
use crate::proxy::handlers::StaticFileHandler;
use crate::route::RouteRuntime;
use crate::runtime::{RuntimeState, UpstreamRuntime};

use crate::traffic_management::{
    AdmissionGuard, SelectedUpstream, ServiceId, TrafficDirector, TrafficManager, UpstreamOutcome,
};
use crate::ws_connection_management::WsConnectionManager;
use arc_swap::ArcSwap;
use async_trait::async_trait;
use http::{StatusCode, Version, header};
use pingora::prelude::*;
use pingora_http::{RequestHeader, ResponseHeader};
use std::sync::Arc;

/// PublicGateway is the core orchestration abstraction in Snakeway.
/// It wraps Pingora hooks and applies traffic decisions and device lifecycle hooks.
pub struct PublicGateway {
    gw_ctx: GatewayCtx,
    traffic_director: TrafficDirector,

    // Handler(s)
    static_file_handler: StaticFileHandler,
}

impl PublicGateway {
    pub fn new(
        state: Arc<ArcSwap<RuntimeState>>,
        traffic_manager: Arc<TrafficManager>,
        connection_manager: Arc<WsConnectionManager>,
    ) -> Self {
        let gw_ctx = GatewayCtx::new(state, traffic_manager.clone(), connection_manager);
        Self {
            gw_ctx,
            traffic_director: TrafficDirector,
            static_file_handler: StaticFileHandler,
        }
    }
}

/// Pingora hook execution order in ProxyHttp...
///
/// This is a giant orchestration traint implementation, so better to lay this out explicitly,
/// especially because it might change in later Pingora versions.
///
/// 1. new_ctx()
///    - Allocate empty RequestCtx
///
/// 2. request_filter()
///    - Hydrate ctx from Session
///    - Run on_request devices
///    - Route match (static vs proxy)
///    - Static responses end here
///
/// 3. upstream_peer()
///    - Select upstream (TrafficDirector)
///    - Circuit admission decision
///    - Create AdmissionGuard if admitted
///    - Construct HttpPeer
///
/// 4. upstream_request_filter()
///    - Run before_proxy devices
///    - Finalize upstream request headers/method
///
/// 5. [Pingora upstream I/O]
///    - Connect, TLS, send request, receive response
///
/// 6. upstream_response_filter()
///    - Run after_proxy devices
///    - Mutate response headers/status
///    - Detect WS upgrade (on_ws_open)
///
/// 7. response_filter()
///    - Run on_response devices
///    - Classify HTTP outcome (success / 5xx)
///
/// 8. logging()   /// ALWAYS LAST
///    - Capture transport errors
///    - Run on_ws_close if needed
///    - Finalize AdmissionGuard (circuit success/failure)
#[async_trait]
impl ProxyHttp for PublicGateway {
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

        let selected_upstream = self.select_upstream(ctx, &state, &service_id, service_name)?;
        let upstream = selected_upstream.upstream;

        let mut peer = match upstream {
            UpstreamRuntime::Tcp(tcp) => Ok(HttpPeer::new(
                tcp.http_peer_addr(),
                tcp.use_tls,
                tcp.sni.clone(),
            )),
            UpstreamRuntime::Unix(unix) => {
                HttpPeer::new_uds(&unix.path, unix.use_tls, unix.sni.clone()).map_err(|e| {
                    anyhow::anyhow!(
                        "Could not connect to unix domain socket `{}`: {}",
                        unix.path,
                        e
                    )
                })
            }
        }
        .map_err(|_| Error::new(Custom("http peer creation failed")))?;

        // Enforce protocol rules for this upstream and request.
        self.enforce_protocol(&mut peer, ctx, upstream)?;

        // Set upstream authority for gRPC requests.
        if let UpstreamRuntime::Tcp(tcp) = upstream {
            let authority = format!("{}:{}", tcp.host, tcp.port);
            ctx.upstream_authority = Some(authority);
        }

        // Record that this request was admitted by the circuit breaker.
        // The TrafficDirector already called `circuit_allows` for selection.
        ctx.cb_started = selected_upstream.cb_started;

        if ctx.cb_started {
            let guard = AdmissionGuard::new(
                self.gw_ctx.traffic_manager.clone(),
                service_id.clone(),
                upstream.id(),
            );

            ctx.admission_guard = Some(guard);
        }

        ctx.selected_upstream = Some((service_id, upstream.id()));

        Ok(Box::new(peer))
    }

    /// ACCEPT → INSPECT → ROUTE → (RESPOND | PROXY)
    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        // The request ctx existed before now but had no data.
        ctx.hydrate_from_session(session);
        debug_assert!(ctx.method.is_some());
        debug_assert!(ctx.original_uri.is_some());

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
            RouteRuntime::Static { id, .. } => {
                ctx.route_id = Some(id.clone());
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

            RouteRuntime::Service {
                id,
                upstream,
                allow_websocket,
                ws_max_connections,
                ..
            } => {
                ctx.route_id = Some(id.clone());

                // If it is a websocket upgrade request, check if the upstream supports websockets.
                if ctx.is_upgrade_req {
                    if !allow_websocket {
                        session
                            .respond_error(StatusCode::UPGRADE_REQUIRED.as_u16())
                            .await?;
                        return Ok(true);
                    }

                    // Acquire a connection slot for ws guard.
                    let guard = self
                        .gw_ctx
                        .connection_manager
                        .try_acquire(id, ws_max_connections.to_owned())
                        .ok_or_else(|| Error::new(Custom("too many websocket connections")))?;

                    ctx.ws_guard = Some(guard);
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
                let Some(method) = ctx.method.clone() else {
                    return Err(Error::new(Custom("request method missing")));
                };
                upstream.set_method(method);
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
        // It may seem odd to put this in a "logging" hook, but it is the only way to do it.
        // Pingora guarantees the logging hook is called last, which is the best that can be
        // done in Pingora 0.6.0.
        if ctx.ws_opened {
            DevicePipeline::run_on_ws_close(
                self.gw_ctx.state().devices.all(),
                &WsCloseCtx::default(),
            );
        }

        // Capture transport-level failure.
        if let Some(err) = e {
            ctx.upstream_outcome = Some(UpstreamOutcome::Transport(classify_pingora_error(err)));
        }

        // Finalize request guard...
        self.finalize_admission_guard(ctx);
    }
}

impl PublicGateway {
    /// Select an upstream for the given request.
    fn select_upstream<'a>(
        &self,
        ctx: &RequestCtx,
        state: &'a RuntimeState,
        service_id: &ServiceId,
        service_name: &str,
    ) -> std::result::Result<SelectedUpstream<'a>, BError> {
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
            .find(|u| u.id() == decision.upstream_id)
            .ok_or_else(|| Error::new(Custom("selected upstream not found")))?;

        Ok(SelectedUpstream {
            upstream,
            cb_started: decision.cb_started,
        })
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
            if !upstream.use_tls() {
                return Err(Error::new(Custom("gRPC upstream must use TLS and HTTP/2")));
            }
            peer.options.set_http_version(2, 2);
        }
        Ok(())
    }

    /// Finalizes the request guard by reporting success or failure to the traffic manager.
    ///
    /// This method determines the outcome of the request based on the upstream response
    /// and circuit breaker configuration. It marks the request as successful or failed,
    /// which updates the circuit breaker state for the selected upstream.
    ///
    /// Success criteria:
    /// - No transport error occurred
    /// - HTTP status < 500 (if count_http_5xx_as_failure is true)
    /// - Any status code (if count_http_5xx_as_failure is false)
    ///
    /// This is called from the logging hook to ensure it runs after all other processing.
    fn finalize_admission_guard(&self, ctx: &mut RequestCtx) {
        let (service_id, _) = match ctx.selected_upstream.as_ref() {
            Some(v) => v,
            None => return,
        };

        let guard = match ctx.admission_guard.as_mut() {
            Some(g) => g,
            None => return,
        };

        let success = match ctx.upstream_outcome {
            Some(UpstreamOutcome::Transport(_)) => false,

            Some(UpstreamOutcome::HttpStatus(code)) => {
                let count_5xx = self
                    .gw_ctx
                    .traffic_manager
                    .circuit_params
                    .get(service_id)
                    .map(|p| p.count_http_5xx_as_failure)
                    .unwrap_or(true);

                if count_5xx { code < 500 } else { true }
            }

            Some(UpstreamOutcome::Success) => true,

            None => true,
        };

        if success {
            guard.success();
        } else {
            guard.failure();
        }
    }
}
