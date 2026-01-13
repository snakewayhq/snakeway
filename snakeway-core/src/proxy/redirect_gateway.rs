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

        // Set the redirect destination via the location header.
        let path_and_query = session
            .req_header()
            .uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");
        let location = format!("https://{}{}", self.destination, path_and_query);
        resp.insert_header("Location", &location)?;
        resp.insert_header("Connection", "close")?;
        resp.insert_header("Content-Length", "0")?;

        session.write_response_header(Box::new(resp), true).await?;

        Ok(true)
    }
}
