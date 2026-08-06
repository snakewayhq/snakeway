use crate::proxy::handlers::StaticFileHandler;
use crate::proxy::proxy_ctx::ProxyCtx;
use crate::proxy::traffic::headers::{write_back_request_headers, write_back_response_headers};
use crate::proxy::traffic::smuggle_detection::is_cl_te_smuggling_attempt;
use crate::proxy::traffic::{BodyBytesReceived, DeclaredContentLength, UpstreamResponseSnapshot};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use bytes::Bytes;
use http::{StatusCode, Version, header};
use pingora::http::{RequestHeader, ResponseHeader};
use pingora::prelude::*;
use snakeway_engine::WsConnectionManager;
use snakeway_engine::ctx::{RequestCtx, ResponseCtx, WsCloseCtx, WsCtx};
use snakeway_engine::device::builtin::request_filter::ClientBodyTimeout;
use snakeway_engine::device::core::{DevicePipeline, DeviceResult};
use snakeway_engine::route::RouteRuntime;
use snakeway_engine::runtime::{RuntimeState, UpstreamRuntime};
use snakeway_engine::traffic::{
    AdmissionGuard, ProtocolMode, ServiceId, TrafficDirector, TrafficManager, UpstreamOutcome,
};
use snakeway_observability::{HeaderExtractor, Metrics, RequestHeaderInjector};
use std::sync::Arc;
use std::time::Duration;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// TrafficProxy is the core orchestration abstraction in Snakeway.
/// It wraps Pingora hooks and applies traffic decisions and device lifecycle hooks.
pub(crate) struct TrafficProxy {
    listener: Arc<str>,
    pub(in crate::proxy) proxy_ctx: ProxyCtx,
    pub(in crate::proxy) traffic_director: TrafficDirector,
    static_file_handler: StaticFileHandler,
    upstream_connect_timeout: Option<Duration>,
    upstream_read_timeout: Option<Duration>,
}

impl TrafficProxy {
    pub(crate) fn new(
        listener: Arc<str>,
        state: Arc<ArcSwap<RuntimeState>>,
        traffic_manager: Arc<TrafficManager>,
        connection_manager: Arc<WsConnectionManager>,
        metrics: Option<Arc<Metrics>>,
        upstream_connect_timeout: Option<Duration>,
        upstream_read_timeout: Option<Duration>,
    ) -> Self {
        let proxy_ctx = ProxyCtx::new(state, traffic_manager.clone(), connection_manager, metrics);
        Self {
            listener,
            proxy_ctx,
            traffic_director: TrafficDirector,
            static_file_handler: StaticFileHandler,
            upstream_connect_timeout,
            upstream_read_timeout,
        }
    }
}

/// Pingora hook execution order in ProxyHttp for TrafficProxy
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
impl ProxyHttp for TrafficProxy {
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
        let state = self.proxy_ctx.state();

        let service_name = ctx
            .service
            .as_ref()
            .ok_or_else(|| Error::new(Custom("no service selected")))?;
        let service_id = ServiceId::new(service_name.as_str());

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

        // Apply upstream timeouts.
        // The read timeout is per-read (idle), so it bounds a stalled origin
        // without breaking slow-but-progressing responses.
        // It is skipped for websocket upgrades so idle long-lived connections
        // are not torn down.
        if let Some(t) = self.upstream_connect_timeout {
            // The total_connection_timeout setting bounds the whole connection
            // establishment (TCP connect + TLS handshake).
            // The inner connection_timeout (TCP connect only) is left unset
            // because it would be redundant since the total bound already caps it.
            peer.options.total_connection_timeout = Some(t);
        }
        if let Some(t) = self.upstream_read_timeout
            && !ctx.is_upgrade_req()
        {
            peer.options.read_timeout = Some(t);
        }

        // Resolve the wire protocol once and store it for later hooks.
        let mode = self.enforce_protocol(&mut peer, ctx, upstream)?;
        ctx.protocol_mode = Some(mode);

        // Set upstream authority for end-to-end h2 (gRPC, h2-to-h2).
        if mode == ProtocolMode::Http2EndToEnd {
            ctx.upstream_authority = Some(upstream.authority());
        }

        // Record that this request was admitted by the circuit breaker.
        // The TrafficDirector already called `circuit_allows` for selection.
        ctx.cb_started = selected_upstream.cb_started;

        if ctx.cb_started {
            let guard = AdmissionGuard::new(
                self.proxy_ctx.traffic_manager.clone(),
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
        // Normalization rejections (malformed request, missing Host, SNI
        // mismatch) are client errors, so they get a 400 rather than a 500.
        if let Err(e) = ctx.hydrate_from_session(session) {
            tracing::warn!(error = %e, "request rejected during normalization");
            session
                .respond_error(StatusCode::BAD_REQUEST.as_u16())
                .await?;
            return Ok(true);
        }

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
        let state = self.proxy_ctx.state();

        // Child span covering on_request devices, route matching, and service selection.
        let _routing_span = tracing::info_span!("routing");
        let _routing_enter = _routing_span.enter();

        // Run on_request devices first (applies to both static and upstream requests).
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

    async fn early_request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
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

        let state = self.proxy_ctx.state();
        match DevicePipeline::on_stream_request_body(state.devices.all(), ctx, body, end_of_stream)
        {
            DeviceResult::Continue => Ok(()),
            DeviceResult::Respond(resp) => {
                resp.write_to_session(&mut session.downstream_session).await
            }
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

        let state = self.proxy_ctx.state();

        match DevicePipeline::run_before_proxy(state.devices.all(), ctx) {
            DeviceResult::Continue => {
                // Applies upstream intent derived from the request context.
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
                        let authority = ctx.upstream_authority().ok_or_else(|| {
                            Error::new(Custom("missing upstream authority for h2"))
                        })?;
                        upstream.insert_header(header::HOST, authority)?;
                    }
                    ProtocolMode::Http1 => {
                        if !upstream.headers.contains_key(header::HOST) {
                            // An HTTP/2 downstream request carries its authority in
                            // the `:authority` pseudo-header, which never appears in
                            // the header map rebuilt above.
                            // HTTP/1.1 requires Host (RFC 9112 §3.2), so derive it
                            // from the request authority.
                            let authority = ctx.downstream_authority().ok_or_else(|| {
                                Error::new(Custom("missing authority for h1 upstream Host"))
                            })?;
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
        let mut resp_ctx =
            ResponseCtx::from_request(ctx, upstream.status, upstream.headers.clone());
        let state = self.proxy_ctx.state();

        match DevicePipeline::run_after_proxy(state.devices.all(), &mut resp_ctx) {
            DeviceResult::Continue => {}
            DeviceResult::Respond(_) => {}
            DeviceResult::Error(err) => {
                tracing::warn!("device error after_proxy: {err}");
            }
        }

        upstream.set_status(resp_ctx.status)?;
        write_back_response_headers(upstream, &resp_ctx.headers)?;

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
            DevicePipeline::run_on_ws_open(self.proxy_ctx.state().devices.all(), &WsCtx::default());
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
        if ctx.ws_opened {
            // WebSocket upgrade is a protocol switch, not a response.
            return Ok(());
        }

        let mut resp_ctx =
            ResponseCtx::from_request(ctx, upstream.status, upstream.headers.clone());
        let state = self.proxy_ctx.state();
        match DevicePipeline::run_on_response(state.devices.all(), &mut resp_ctx) {
            DeviceResult::Continue => {}
            DeviceResult::Respond(_) => {}
            DeviceResult::Error(err) => {
                tracing::warn!("device error on_response: {err}");
            }
        }

        upstream.set_status(resp_ctx.status)?;
        write_back_response_headers(upstream, &resp_ctx.headers)?;

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

        let mut resp_ctx = ResponseCtx::from_request(ctx, status, headers);
        let state = self.proxy_ctx.state();

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
        // done in Pingora 0.8.1.
        if ctx.ws_opened {
            DevicePipeline::run_on_ws_close(
                self.proxy_ctx.state().devices.all(),
                &WsCloseCtx::default(),
            );
        }

        self.finalize_request(ctx, e);
    }
}
