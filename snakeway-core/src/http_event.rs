#[derive(Debug, Clone, Copy)]
pub(crate) enum HttpEvent {
    Request,
    BeforeProxy,
    AfterProxy,
    Response,
}

impl HttpEvent {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            HttpEvent::Request => "request",
            HttpEvent::BeforeProxy => "before_proxy",
            HttpEvent::AfterProxy => "after_proxy",
            HttpEvent::Response => "response",
        }
    }
}
