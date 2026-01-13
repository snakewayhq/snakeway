use crate::ctx::RequestCtx;
use async_trait::async_trait;
use pingora::prelude::{HttpPeer, ProxyHttp, Session};
use pingora::{Custom, Error};
use pingora_http::ResponseHeader;

pub struct RedirectGateway {
    destination: String,
    response_code: u16,
}

impl RedirectGateway {
    pub fn new(to: String, response_code: u16) -> Self {
        Self {
            destination: to,
            response_code,
        }
    }
}

#[async_trait]
impl ProxyHttp for RedirectGateway {
    type CTX = RequestCtx;

    fn new_ctx(&self) -> Self::CTX {
        // Minimal ctx - redirect requests never enter the proxy lifecycle.
        RequestCtx::empty()
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        // This is unreachable by design.
        Err(Error::new(Custom(
            "RedirectGateway attempted to proxy upstream (bug)",
        )))
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        // RedirectGateway is terminal: it always handles the request.
        let mut resp = ResponseHeader::build(self.response_code, None)?;
        resp.insert_header("Location", &self.destination)?;

        session.write_response_header(Box::new(resp), true).await?;

        Ok(true)
    }
}
