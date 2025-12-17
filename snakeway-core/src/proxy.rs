use async_trait::async_trait;
use pingora::prelude::*;
use pingora_http::{RequestHeader, ResponseHeader};

use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::pipeline::DevicePipeline;
use crate::device::core::registry::DeviceRegistry;
use crate::device::core::result::DeviceResult;

pub struct SnakewayGateway {
    pub upstream_host: String,
    pub upstream_port: u16,
    pub use_tls: bool,
    pub sni: String,

    pub devices: DeviceRegistry,
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
    /// ACCEPT → INSPECT → DECIDE → (RESPOND | PROXY)
    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<bool> {
        let req = session.req_header();

        *ctx = RequestCtx::new(
            req.method.clone(),
            req.uri.clone(),
            req.headers.clone(),
            Vec::new(),
        );

        match DevicePipeline::run_on_request(self.devices.all(), ctx) {
            DeviceResult::Continue => Ok(false),

            DeviceResult::Respond(resp) => {
                session.respond_error(resp.status.as_u16()).await?;
                Ok(true)
            }

            DeviceResult::Error(err) => {
                tracing::error!("device error in on_request: {err}");
                session.respond_error(500).await?;
                Ok(true)
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
        let mut resp_ctx = ResponseCtx::new(
            upstream.status,
            upstream.headers.clone(),
            Vec::new(),
        );

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
        let mut resp_ctx = ResponseCtx::new(
            upstream.status,
            upstream.headers.clone(),
            Vec::new(),
        );

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
