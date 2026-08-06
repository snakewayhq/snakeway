use crate::proxy::handlers::StaticFileHandler;
use crate::proxy::proxy_ctx::ProxyCtx;
use crate::proxy::traffic::headers::write_back_response_headers;
use crate::proxy::traffic::smuggle_detection::is_cl_te_smuggling_attempt;
use crate::proxy::traffic::upstream_intent::apply_upstream_intent;
use crate::proxy::traffic::{BodyBytesReceived, DeclaredContentLength, UpstreamResponseSnapshot};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use bytes::Bytes;
use http::StatusCode;
use pingora::http::{RequestHeader, ResponseHeader};
use pingora::prelude::*;
use snakeway_engine::WsConnectionManager;
use snakeway_engine::ctx::{RequestCtx, ResponseCtx, WsCloseCtx, WsCtx};
use snakeway_engine::device::core::{DevicePipeline, DeviceResult};
use snakeway_engine::runtime::RuntimeState;
use snakeway_engine::traffic::{
    AdmissionGuard, ServiceId, TrafficDirector, TrafficManager, UpstreamOutcome,
};
use snakeway_observability::Metrics;
use std::sync::Arc;
use std::time::Duration;

/// TrafficProxy is the core orchestration abstraction in Snakeway.
/// It wraps Pingora hooks and applies traffic decisions and device lifecycle hooks.
pub(crate) struct TrafficProxy {
    pub(in crate::proxy) listener: Arc<str>,
    pub(in crate::proxy) proxy_ctx: ProxyCtx,
    pub(in crate::proxy) traffic_director: TrafficDirector,
    pub(in crate::proxy) static_file_handler: StaticFileHandler,
    pub(in crate::proxy) upstream_connect_timeout: Option<Duration>,
    pub(in crate::proxy) upstream_read_timeout: Option<Duration>,
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
        let _root_span = ctx.request_span.clone();
        let _root = _root_span.as_ref().map(|s| s.enter());
        let _selection_span = tracing::info_span!("upstream_selection").entered();
        let state = self.proxy_ctx.state();

        let service_name = ctx
            .service
            .as_ref()
            .ok_or_else(|| Error::new(Custom("no service selected")))?;
        let service_id = ServiceId::new(service_name.as_str());

        let selected_upstream = self.select_upstream(ctx, &state, &service_id, service_name)?;
        let upstream = selected_upstream.upstream;

        let peer = self.build_peer(ctx, upstream)?;

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

        self.open_request_span(session, ctx);
        let _span = ctx.request_span.clone();
        let _enter = _span.as_ref().map(|s| s.enter());

        let state = self.proxy_ctx.state();

        // Child span covering on_request devices, route matching, and service selection.
        let _routing_span = tracing::info_span!("routing");
        let _routing_enter = _routing_span.enter();

        if self.run_on_request_devices(session, ctx, &state).await? {
            return Ok(true);
        }

        self.dispatch_route(session, ctx, &state).await
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
            DeviceResult::Continue => apply_upstream_intent(upstream, ctx),

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
