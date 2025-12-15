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
    /// ACCEPT --> INSPECT --> DECIDE --> REWRITE
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

            DeviceResult::ShortCircuit(resp) => {
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
    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        match DevicePipeline::run_before_proxy(self.devices.all(), ctx) {
            DeviceResult::Continue | DeviceResult::ShortCircuit(_) | DeviceResult::Error(_) => {
                // Apply upstream intent explicitly
                upstream.set_method(ctx.method.clone());

                let path = ctx.upstream_path();
                upstream.set_uri(path.parse().unwrap());

                Ok(())
            }
        }
    }

    /// Snakeway `after_proxy` --> Pingora `upstream_response_filter`
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
            DeviceResult::Continue | DeviceResult::ShortCircuit(_) | DeviceResult::Error(_) => {
                upstream.set_status(resp_ctx.status)?;
                Ok(())
            }
        }
    }

    /// Snakeway `on_response` --> Pingora `response_filter`
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
            DeviceResult::Continue | DeviceResult::ShortCircuit(_) | DeviceResult::Error(_) => {
                upstream.set_status(resp_ctx.status)?;
                Ok(())
            }
        }
    }
}
