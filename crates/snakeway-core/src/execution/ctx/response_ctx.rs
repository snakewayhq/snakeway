use http::{HeaderMap, StatusCode, header};
use pingora::http::ResponseHeader;
use pingora::protocols::http::ServerSession;

#[derive(Debug)]
pub struct ResponseCtx {
    pub(crate) request_id: Option<String>,
    pub(crate) status: StatusCode,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Vec<u8>,
}

impl ResponseCtx {
    pub(crate) fn new(
        request_id: Option<String>,
        status: StatusCode,
        headers: HeaderMap,
        body: Vec<u8>,
    ) -> Self {
        Self {
            request_id,
            status,
            headers,
            body,
        }
    }

    pub(crate) fn forbidden(request_id: Option<String>) -> Self {
        Self::new(
            request_id,
            StatusCode::FORBIDDEN,
            HeaderMap::new(),
            b"Forbidden".to_vec(),
        )
    }

    pub(crate) fn too_many_requests(request_id: Option<String>) -> Self {
        Self::new(
            request_id,
            StatusCode::TOO_MANY_REQUESTS,
            HeaderMap::new(),
            b"Too many requests".to_vec(),
        )
    }

    pub(crate) async fn write_to_session(self, session: &mut ServerSession) -> pingora::Result<()> {
        let mut response = ResponseHeader::build(self.status, None)?;
        for (name, value) in &self.headers {
            response.append_header(name.clone(), value)?;
        }
        if !self.body.is_empty() {
            response.insert_header(header::CONTENT_LENGTH, self.body.len().to_string())?;
        }
        let end_of_body = self.body.is_empty();
        session.write_response_header(Box::new(response)).await?;
        if !self.body.is_empty() {
            session.write_response_body(self.body.into(), true).await?;
        }
        Ok(())
    }
}
