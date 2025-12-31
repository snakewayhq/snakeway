use pingora_http::RequestHeader;

pub enum RequestKind {
    Admin { path: String },
    Normal,
}

pub fn classify_request(req: &RequestHeader) -> RequestKind {
    let path = req.uri.path();

    if path.starts_with("/admin/") {
        RequestKind::Admin {
            path: path.to_string(),
        }
    } else {
        RequestKind::Normal
    }
}
