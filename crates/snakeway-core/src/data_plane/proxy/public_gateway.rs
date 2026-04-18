use crate::control_plane::observability::{HeaderExtractor, Metrics, RequestHeaderInjector};
use crate::data_plane::proxy::error_classification::classify_pingora_error;
use crate::data_plane::proxy::gateway_ctx::GatewayCtx;
use crate::data_plane::proxy::handlers::StaticFileHandler;
use crate::data_plane::ws_connection_management::WsConnectionManager;
use crate::execution::ctx::{RequestCtx, RequestId, ResponseCtx, WsCloseCtx, WsCtx};
use crate::execution::device::builtin::request_filter::ClientBodyTimeout;
use crate::execution::device::core::{DevicePipeline, DeviceResult};
use crate::execution::route::RouteRuntime;
use crate::execution::traffic::{
    AdmissionGuard, SelectedUpstream, ServiceId, TrafficDirector, TrafficManager, TransportFailure,
    UpstreamOutcome,
};
use crate::runtime::{RuntimeState, UpstreamRuntime};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use bytes::Bytes;
use http::{HeaderMap, StatusCode, Version, header};
use opentelemetry::KeyValue;
use pingora::http::{RequestHeader, ResponseHeader};
use pingora::prelude::*;
use pingora::protocols::http::ServerSession;
use std::sync::Arc;
use std::time::Duration;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// PublicGateway is the core orchestration abstraction in Snakeway.
/// It wraps Pingora hooks and applies traffic decisions and device lifecycle hooks.
pub(crate) struct PublicGateway {
    listener: Arc<str>,
    gw_ctx: GatewayCtx,
    traffic_director: TrafficDirector,
    static_file_handler: StaticFileHandler,
}

impl PublicGateway {
    pub(crate) fn new(
        listener: Arc<str>,
        state: Arc<ArcSwap<RuntimeState>>,
        traffic_manager: Arc<TrafficManager>,
        connection_manager: Arc<WsConnectionManager>,
        metrics: Option<Arc<Metrics>>,
    ) -> Self {
        let gw_ctx = GatewayCtx::new(state, traffic_manager.clone(), connection_manager, metrics);
        Self {
            listener,
            gw_ctx,
            traffic_director: TrafficDirector,
            static_file_handler: StaticFileHandler,
        }
    }
}

/// Detects CL.TE / TE.CL smuggling attempts that Pingora's HTTP/1 parser has partially handled.
///
/// When a request carries both `Content-Length` and `Transfer-Encoding`, Pingora strips CL
/// (RFC 9112 §6.3) and disables keepalive on the session (RFC 9112 §6.1-15). Since CL is gone
/// by the time we run, we infer CL+TE from the keepalive flag instead.
///
/// For an HTTP/1.1 request that didn't send `Connection: close` and still has reuse budget,
/// Pingora leaves keepalive on by default. Keepalive being off under those conditions means
/// the CL+TE detection path fired. We read that via `ServerSession::H1.will_keepalive()`.
///
/// We filter out the other keepalive-off cases that would false-positive:
///   * HTTP/1.0 — defaults to keepalive-off (1.0 + TE is already rejected upstream anyway).
///   * Exhausted reuse counter — `will_keepalive()` is also false when reuses_remaining == 0.
///
/// Caveat: this relies on Pingora's current internals, not a stable API. Revisit on upgrade.
fn is_cl_te_smuggling_attempt(session: &Session) -> bool {
    let req = session.req_header();

    // Only HTTP/1.1-1.0 defaults to keepalive-off and would false-positive,
    // and 1.0 + TE is already rejected by Pingora's validate_request.
    if req.version != Version::HTTP_11 {
        return false;
    }

    if !req.headers.contains_key("transfer-encoding") {
        return false;
    }

    let client_closed = req
        .headers
        .get("connection")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(',')
        .any(|t| t.trim().eq_ignore_ascii_case("close"));

    if client_closed {
        return false;
    }

    match session.downstream_session.as_ref() {
        ServerSession::H1(h1) => {
            // Exclude the reuse-counter-exhausted case, which also turns keepalive off.
            if h1.get_keepalive_reuses_remaining() == Some(0) {
                return false;
            }
            !h1.will_keepalive()
        }
        _ => false,
    }
}

/// Pingora hook execution order in ProxyHttp for PublicGateway
///
/// This is a giant orchestration trait implementation, so better to lay this out explicitly,
/// especially because it might change in later Pingora versions.
///
/// Hooks related to caching, custom forwarding, and subrequest spawning are omitted
/// because Snakeway does not use those Pingora features.
///
/// 1. new_ctx()
///    - Allocate empty RequestCtx
///
/// 2. [unused] early_request_filter()
///    - Earliest hook, runs before downstream modules
///
/// 3. request_filter()
///    - Hydrate ctx from Session
///    - Run on_request devices
///    - Route match (static vs proxy)
///    - Static responses end here
///
/// 4. [unused] proxy_upstream_filter()
///    - Final decision whether request is allowed upstream
///    - May short-circuit with a response
///
/// 5. upstream_peer()
///    - Select upstream (TrafficDirector)
///    - Circuit admission decision
///    - Create AdmissionGuard if admitted
///    - Construct HttpPeer
///
/// 6. [unused] connected_to_upstream()
///    - Called after TCP/TLS connection is established or reused
///
/// 7. upstream_request_filter()
///    - Set HTTP/2 :authority pseudo-header for gRPC
///    - Run before_proxy devices (header mutation, path rewriting)
///    - Apply upstream method/path intent from RequestCtx
///
/// 8. request_body_filter()
///    - Run on_stream_request_body devices on each request body chunk
///    - Validate Content-Length against actual bytes received
///
/// 9. [Pingora upstream I/O]
///    - Send request, receive response
///
/// 10. upstream_response_filter()
///     - Run after_proxy devices
///     - Mutate response headers/status
///     - Snapshot response for body filter
///
/// 11. upstream_response_body_filter()
///     - Run on_stream_response_body devices on each upstream response body chunk
///
/// 12. [unused] upstream_response_trailer_filter()
///     - Inspect/modify upstream response trailers
///
/// 13. response_filter()
///     - Run on_response devices (response header mutation)
///     - Determine upstream outcome (success / HTTP 5xx) for circuit breaker
///
/// 14. [unused] response_body_filter()
///     - Inspect/modify response body chunks before sending to downstream
///
/// 15. [unused] response_trailer_filter()
///     - Inspect/modify response trailers before sending to downstream
///
/// `Error path hooks (not in normal flow)`
///
/// 16. [unused] error_while_proxy()
///     - Called if upstream fails mid-stream
///
/// 17. [unused] fail_to_connect()
///     - Called if upstream connection cannot be established
///
/// 18. [unused] fail_to_proxy()
///     - Final error handling hook after retries exhausted
///
/// 19. [unused] suppress_error_log()
///     - Decide whether Pingora logs proxy failure
///
/// `Always runs`
///
/// 20. logging()
///     - Capture transport errors
///     - Run on_ws_close if needed
///     - Finalize AdmissionGuard (circuit success/failure)

#[hotpath::measure_all]
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
        let _root = ctx.request_span.as_ref().map(|s| s.enter());
        let _selection_span = tracing::info_span!("upstream_selection").entered();
        let state = self.gw_ctx.state();

        let service_name = ctx
            .service
            .as_ref()
            .ok_or_else(|| Error::new(Custom("no service selected")))?;
        let service_id = ServiceId(service_name.clone());

        let selected_upstream = self.select_upstream(ctx, &state, &service_id, service_name)?;
        let upstream = selected_upstream.upstream;

        // Creating an HttpPeer instance per request may raise an eyebrow, but
        // it is merely a sort of configuration object that is used by Pingora
        // to compute a hash later when its internal pooling logic runs.
        let mut peer = match upstream {
            UpstreamRuntime::Tcp(tcp) => {
                let mut peer = HttpPeer::new(tcp.http_peer_addr(), tcp.use_tls, tcp.sni.clone());
                if tcp.use_tls {
                    // Wire-up per-upstream TLS settings.
                    peer.options.verify_cert = tcp.verify;
                    peer.options.verify_hostname = tcp.verify;
                    if tcp.verify {
                        peer.options.ca = tcp.ca.clone();
                        peer.group_key = tcp.group_key;
                    }
                }
                Ok(peer)
            }
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

        // Set upstream authority for gRPC and http/2.0 requests.
        if ctx.is_http2() {
            ctx.upstream_authority = Some(upstream.authority());
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
        ctx.hydrate_from_session(session).map_err(|e| {
            tracing::warn!(error = %e, "request rejected during normalization");
            e.as_pingora_error()
        })?;

        // Extract W3C Trace Context from downstream request headers.
        // When no traceparent header is present, the context is empty and
        // set_parent below becomes a no-op (the span stays a root).
        let parent_cx = opentelemetry::global::get_text_map_propagator(|prop| {
            prop.extract(&HeaderExtractor(&session.req_header().headers))
        });

        // Setup request root span and add it to the request context.
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
        let _span = ctx.request_span.clone();
        let _enter = _span.as_ref().map(|s| s.enter());

        // Grab state.
        let state = self.gw_ctx.state();

        // Child span covering on_request devices, route matching, and service selection.
        let _routing_span = tracing::info_span!("routing");
        let _routing_enter = _routing_span.enter();

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

        // Make a decision about the route.
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

    async fn early_request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
        // Hydrate request context from session.
        // RFC 9112 §6.3: Reject CL.TE / TE.CL request smuggling attempts.
        //
        // Pingora's HTTP/1 parser strips Content-Length when both CL and Transfer-Encoding
        // are present (RFC 9112 §6.3) and disables keepalive (RFC 9112 §6.1-15), but does
        // not itself reject the request.  By the time `early_request_filter` runs, the CL header
        // is already gone, so our header-normalization layer cannot see both headers.
        //
        // We detect the stripping by checking that:
        //   1. The request is HTTP/1.x
        //   2. Transfer-Encoding is present (Pingora keeps it)
        //   3. The client did not explicitly send Connection: close (which would legitimately
        //      disable keepalive for an unrelated reason)
        //   4. Pingora nonetheless disabled keepalive — the only remaining cause is CL+TE
        if is_cl_te_smuggling_attempt(session) {
            tracing::warn!("request rejected: CL.TE smuggling attempt detected");
            session.respond_error(400).await?;
        }
        Ok(())
    }

    /// A method to filter and process the request body during a streaming session.
    /// This method is currently used for running device pipeline operations on the request body.
    ///
    /// Additionally, when `end_of_stream` is true and a `Content-Length` header was
    /// declared, the total bytes received are compared against the declared value.
    /// A mismatch means the client closed the connection (or timed out) before
    /// sending the full body — forwarding a truncated body to the upstream would
    /// waste backend resources or cause incorrect behaviour.
    async fn request_body_filter(
        &self,
        session: &mut Session,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        let _span = ctx.request_span.clone();
        let _enter = _span.as_ref().map(|s| s.enter());

        // Track total body bytes received.
        if let Some(chunk) = body.as_ref() {
            let counter = ctx
                .extensions
                .get_mut::<BodyBytesReceived>()
                .map(|c| {
                    c.0 += chunk.len() as u64;
                    c.0
                })
                .unwrap_or_else(|| {
                    let len = chunk.len() as u64;
                    ctx.extensions.insert(BodyBytesReceived(len));
                    len
                });
            // Use the counter to avoid an unused-variable warning.
            let _ = counter;
        }

        // Content-Length underflow check: if the stream has ended and the
        // client declared a Content-Length, verify that the full body arrived.
        if end_of_stream && let Some(&DeclaredContentLength(declared)) = ctx.extensions.get() {
            let received = ctx.extensions.get::<BodyBytesReceived>().map_or(0, |c| c.0);
            if received < declared {
                tracing::warn!(
                    request_id = ctx.request_id(),
                    declared,
                    received,
                    "request body underflow: client sent fewer bytes than Content-Length"
                );
                session.respond_error(400).await?;
                return Err(Error::new(Custom(
                    "request body shorter than Content-Length",
                )));
            }
        }

        let state = self.gw_ctx.state();
        match DevicePipeline::on_stream_request_body(state.devices.all(), ctx, body, end_of_stream)
        {
            DeviceResult::Continue => Ok(()),
            DeviceResult::Respond(resp) => session.respond_error(resp.status.as_u16()).await,
            DeviceResult::Error(err) => {
                tracing::error!("device error on_stream_request_body: {err}");
                Err(Error::new(Custom("device error on_stream_request_body")))
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
        let _root_span = ctx.request_span.clone();
        let _root = _root_span.as_ref().map(|s| s.enter());
        let _req_span = tracing::info_span!("upstream_request");
        let _req_enter = _req_span.enter();

        if upstream.version == Version::HTTP_2 {
            let authority = ctx
                .upstream_authority()
                .ok_or_else(|| Error::new(Custom("missing upstream authority for h2")))?;

            // Set Host - Pingora will map it to :authority
            upstream.insert_header(header::HOST, authority)?;
        }

        let state = self.gw_ctx.state();

        match DevicePipeline::run_before_proxy(state.devices.all(), ctx) {
            DeviceResult::Continue => {
                // Applies upstream intent derived from the request context.
                upstream.set_method(ctx.method().to_owned());
                upstream.set_uri(ctx.upstream_path().parse().unwrap());

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
    async fn upstream_response_filter(
        &self,
        _session: &mut Session,
        upstream: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        let _root = ctx.request_span.as_ref().map(|s| s.enter());
        let _resp_span = tracing::info_span!("upstream_response").entered();
        let request_id = ctx.extensions.get::<RequestId>().map(|id| id.0.clone());
        let mut resp_ctx = ResponseCtx::new(
            request_id,
            upstream.status,
            upstream.headers.clone(),
            Vec::new(),
        );
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

        ctx.extensions.insert(UpstreamResponseSnapshot {
            status: upstream.status,
            headers: upstream.headers.clone(),
        });

        if ctx.is_upgrade_req() && upstream.status == StatusCode::SWITCHING_PROTOCOLS {
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
        let _root = ctx.request_span.as_ref().map(|s| s.enter());
        let _resp_span = tracing::info_span!("response").entered();
        if ctx.ws_opened || ctx.is_http2() {
            // Do not run on_response devices for WebSockets or HTTP/2.
            // For WebSockets and HTTP/2, this is not a real "response."
            // For WebSockets, it is a protocol switch.
            return Ok(());
        }

        let request_id = ctx.extensions.get::<RequestId>().map(|id| id.0.clone());
        let mut resp_ctx = ResponseCtx::new(
            request_id,
            upstream.status,
            upstream.headers.clone(),
            Vec::new(),
        );
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

    /// Snakeway `on_stream_response_body` --> Pingora `upstream_response_body_filter`
    ///
    /// Intent:
    /// INSPECT RESPONSE BODY CHUNKS AS THEY STREAM
    fn upstream_response_body_filter(
        &self,
        _session: &mut Session,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> Result<Option<Duration>> {
        let _root = ctx.request_span.as_ref().map(|s| s.enter());

        let snapshot = ctx.extensions.get::<UpstreamResponseSnapshot>();
        let (status, headers) = match snapshot {
            Some(s) => (s.status, s.headers.clone()),
            None => return Ok(None),
        };

        let request_id = ctx.extensions.get::<RequestId>().map(|id| id.0.clone());
        let mut resp_ctx = ResponseCtx::new(request_id, status, headers, Vec::new());
        let state = self.gw_ctx.state();

        match DevicePipeline::on_stream_response_body(
            state.devices.all(),
            &mut resp_ctx,
            body,
            end_of_stream,
        ) {
            DeviceResult::Continue => Ok(None),
            DeviceResult::Respond(_) => Ok(None),
            DeviceResult::Error(err) => {
                tracing::error!("device error on_stream_response_body: {err}");
                Err(Error::new(Custom("device error on_stream_response_body")))
            }
        }
    }

    /// The final step in the Pingora request/response pipeline.
    /// This function is primarily intended for logging,
    /// but it is also used for finalizing request guards.
    async fn logging(&self, _session: &mut Session, e: Option<&Error>, ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        let _span = ctx.request_span.clone();
        let _enter = _span.as_ref().map(|s| s.enter());

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
        if let Some(err) = e
            && let Some(failure) = classify_pingora_error(err)
        {
            ctx.upstream_outcome = Some(UpstreamOutcome::Transport(failure));
        }
        // Finalize request guard...
        self.finalize_admission_guard(ctx);

        // Record metrics (no-op when OTel is disabled).
        self.record_metrics(ctx);
    }
}

impl PublicGateway {
    fn record_metrics(&self, ctx: &RequestCtx) {
        use crate::execution::traffic::circuit::CircuitState;

        let Some(metrics) = &self.gw_ctx.metrics else {
            return;
        };

        let service = ctx.service.as_deref().unwrap_or("unknown");
        let route = ctx
            .route_id
            .as_ref()
            .map(|r| r.as_str())
            .unwrap_or_else(|| "unknown".into());
        let method = ctx.method_str();

        let status = match &ctx.upstream_outcome {
            Some(UpstreamOutcome::Success) => "2xx",
            Some(UpstreamOutcome::HttpStatus(s)) if *s >= 500 => "5xx",
            Some(UpstreamOutcome::HttpStatus(s)) if *s >= 400 => "4xx",
            Some(UpstreamOutcome::HttpStatus(_)) => "other",
            Some(UpstreamOutcome::Transport(_)) => "transport_error",
            None => "no_upstream",
        };

        let request_attrs = &[
            KeyValue::new("method", method.to_string()),
            KeyValue::new("status", status),
            KeyValue::new("service", service.to_string()),
            KeyValue::new("route", route),
        ];

        metrics.http_requests.add(1, request_attrs);

        // Duration and upstream-scoped metrics.
        if let Some((service_id, upstream_id)) = &ctx.selected_upstream {
            let duration_ms = ctx.request_start.elapsed().as_secs_f64() * 1000.0;
            let upstream_str = upstream_id.0.to_string();
            let upstream_attrs = &[
                KeyValue::new("service", service_id.0.clone()),
                KeyValue::new("upstream", upstream_str.clone()),
            ];

            metrics
                .http_request_duration
                .record(duration_ms, upstream_attrs);

            // Error counter.
            match &ctx.upstream_outcome {
                Some(UpstreamOutcome::HttpStatus(s)) if *s >= 500 => {
                    metrics.http_errors.add(
                        1,
                        &[
                            KeyValue::new("service", service_id.0.clone()),
                            KeyValue::new("upstream", upstream_str.clone()),
                            KeyValue::new("error.type", "http_5xx"),
                        ],
                    );
                }
                Some(UpstreamOutcome::Transport(failure)) => {
                    let error_type = match failure {
                        TransportFailure::Connect => "connect",
                        TransportFailure::Timeout => "timeout",
                        TransportFailure::Reset => "reset",
                        TransportFailure::Protocol => "protocol",
                        TransportFailure::Tls => "tls",
                    };
                    metrics.http_errors.add(
                        1,
                        &[
                            KeyValue::new("service", service_id.0.clone()),
                            KeyValue::new("upstream", upstream_str.clone()),
                            KeyValue::new("error.type", error_type),
                        ],
                    );
                }
                _ => {}
            }

            // Gauge: active requests.
            let tm = &self.gw_ctx.traffic_manager;
            metrics
                .upstream_active_requests
                .record(tm.active_requests(service_id, upstream_id), upstream_attrs);

            // Gauge: health status.
            let healthy = tm.health_status(service_id, upstream_id).healthy;
            metrics
                .upstream_health
                .record(u64::from(healthy), upstream_attrs);

            // Gauge: circuit breaker state.
            if let Some(cb) = tm.circuit.get(&(service_id.clone(), *upstream_id)) {
                let state_value = match cb.state() {
                    CircuitState::Closed => 0,
                    CircuitState::Open => 1,
                    CircuitState::HalfOpen => 2,
                };
                metrics
                    .circuit_breaker_state
                    .record(state_value, upstream_attrs);
            }
        }
    }

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

        tracing::info!("decision reason: {}", decision.reason);

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
    pub(crate) fn enforce_protocol(
        &self,
        peer: &mut HttpPeer,
        ctx: &RequestCtx,
        upstream: &UpstreamRuntime,
    ) -> Result<(), BError> {
        if ctx.is_upgrade_req() {
            // WebSockets MUST be HTTP/1.1
            peer.options.set_http_version(1, 1);
        } else if ctx.is_http2() {
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
            Some(UpstreamOutcome::Transport(failure)) => {
                tracing::debug!(
                    service = %service_id,
                    failure = ?failure,
                    "upstream transport failure"
                );
                false
            }

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

/// Declared Content-Length from the request headers, stored in extensions
/// during `request_filter` for comparison at end-of-stream.
#[derive(Debug, Clone, Copy)]
struct DeclaredContentLength(u64);

/// Running total of body bytes received from the downstream client,
/// updated per-chunk in `request_body_filter`.
#[derive(Debug, Clone, Copy)]
struct BodyBytesReceived(u64);

/// Snapshot of the upstream response status and headers, stored in extensions
/// during `upstream_response_filter` for use by `upstream_response_body_filter`.
#[derive(Debug, Clone)]
struct UpstreamResponseSnapshot {
    status: StatusCode,
    headers: HeaderMap,
}
