use async_trait::async_trait;
use pingora::prelude::*;

/// Simple "one upstream" gateway
pub struct SnakewayGateway {
    pub upstream_host: String,
    pub upstream_port: u16,
    pub use_tls: bool,
    pub sni: String,
}

#[async_trait]
impl ProxyHttp for SnakewayGateway {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>, Box<Error>> {
        // A basic fixed upstream.
        let addr = (self.upstream_host.as_str(), self.upstream_port);

        log::info!("Snakeway: connecting to upstream {:?}", addr);

        let peer = HttpPeer::new(addr, self.use_tls, self.sni.clone());
        Ok(Box::new(peer))
    }
}
