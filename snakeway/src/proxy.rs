use async_trait::async_trait;
use pingora::prelude::*;
use pingora_http::{RequestHeader, ResponseHeader};

use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::pipeline::DevicePipeline;
use crate::device::core::registry::DeviceRegistry;
use crate::device::core::result::DeviceResult;

/// Simple "one upstream" gateway
pub struct SnakewayGateway {
    pub upstream_host: String,
    pub upstream_port: u16,
    pub use_tls: bool,
    pub sni: String,

    pub devices: DeviceRegistry,
}

#[async_trait]
impl ProxyHttp for SnakewayGateway {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    /// Simple upstream peer selection
    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let addr = (self.upstream_host.as_str(), self.upstream_port);

        log::info!("Snakeway: connecting to upstream {:?}", addr);

        let peer = HttpPeer::new(addr, self.use_tls, self.sni.clone());
        Ok(Box::new(peer))
    }

    /// Snakeway `on_request` --> Pingora `request_filter`
    async fn request_filter(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<bool> {
        let req = session.req_header();

        let mut ctx = RequestCtx::new(
            req.method.clone(),
            req.uri.clone(),
            req.headers.clone(),
            Vec::new(), // no body handling yet
        );

        match DevicePipeline::run_on_request(self.devices.all(), &mut ctx) {
            DeviceResult::Continue => Ok(false),

            DeviceResult::ShortCircuit(_resp_ctx) => {
                // 403 = "blocked by device" for now
                session.respond_error(403).await?;
                Ok(true)
            }

            DeviceResult::Error(err) => {
                // Log and send a 500
                log::error!("Snakeway device error in on_request: {err}");
                session.respond_error(500).await?;
                Ok(true)
            }
        }
    }

    /// Snakeway `before_proxy` --> Pingora `upstream_request_filter`
    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        let mut ctx = RequestCtx::new(
            upstream_request.method.clone(),
            upstream_request.uri.clone(),
            upstream_request.headers.clone(),
            Vec::new(),
        );

        match DevicePipeline::run_before_proxy(self.devices.all(), &mut ctx) {
            DeviceResult::Continue | DeviceResult::ShortCircuit(_) | DeviceResult::Error(_) => {
                // Propagate method/URI changes
                // Note: Headers are effectively read-only in this hook
                upstream_request.set_method(ctx.method);
                upstream_request.set_uri(ctx.uri);
                Ok(())
            }
        }
    }

    /// Snakeway `after_proxy` --> Pingora `upstream_response_filter`
    fn upstream_response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        let mut ctx = ResponseCtx::new(
            upstream_response.status,
            upstream_response.headers.clone(),
            Vec::new(),
        );

        match DevicePipeline::run_after_proxy(self.devices.all(), &mut ctx) {
            DeviceResult::Continue | DeviceResult::ShortCircuit(_) | DeviceResult::Error(_) => {
                // Can update status: this is supported in Pingora 0.6.x
                upstream_response.set_status(ctx.status)?;
                // Headers remain read-only in this phase
                Ok(())
            }
        }
    }

    /// Snakeway `on_response` --> Pingora `response_filter`
    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        let mut ctx = ResponseCtx::new(
            upstream_response.status,
            upstream_response.headers.clone(),
            Vec::new(),
        );

        match DevicePipeline::run_on_response(self.devices.all(), &mut ctx) {
            DeviceResult::Continue | DeviceResult::ShortCircuit(_) | DeviceResult::Error(_) => {
                upstream_response.set_status(ctx.status)?;
                Ok(())
            }
        }
    }
}
