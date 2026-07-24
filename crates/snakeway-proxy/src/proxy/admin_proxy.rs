use crate::proxy::handlers::{AdminContext, AdminHandler};
use crate::reload::ReloadHandle;
use async_trait::async_trait;
use pingora::prelude::{HttpPeer, ProxyHttp, Session};
use pingora::{Custom, Error};
use snakeway_acme::CertManager;
use snakeway_conf::types::AdminAuthConfig;
use snakeway_engine::WsConnectionManager;
use snakeway_engine::ctx::RequestCtx;
use snakeway_engine::traffic::TrafficManager;
use std::sync::Arc;

pub(crate) struct AdminProxy {
    admin_handler: AdminHandler,
}

impl AdminProxy {
    pub(crate) fn new(
        traffic_manager: Arc<TrafficManager>,
        connection_manager: Arc<WsConnectionManager>,
        reload: Arc<ReloadHandle>,
        cert_manager: Option<Arc<CertManager>>,
        auth: Arc<AdminAuthConfig>,
    ) -> Self {
        let ctx = Arc::new(AdminContext {
            traffic: traffic_manager,
            ws: connection_manager,
            reload,
            certs: cert_manager,
            auth,
        });

        Self {
            admin_handler: AdminHandler::new(ctx),
        }
    }
}

#[async_trait]
impl ProxyHttp for AdminProxy {
    type CTX = RequestCtx;

    fn new_ctx(&self) -> Self::CTX {
        // Minimal ctx - admin requests never enter the proxy lifecycle.
        RequestCtx::empty()
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        // This is unreachable by design.
        Err(Error::new(Custom(
            "AdminProxy attempted to proxy upstream (bug)",
        )))
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        // AdminProxy is terminal: it always handles the request.
        let path = session.req_header().uri.path().to_owned();
        self.admin_handler.handle(session, &path).await
    }
}
