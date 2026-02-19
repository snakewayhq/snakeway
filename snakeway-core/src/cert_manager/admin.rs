use std::time::SystemTime;

#[derive(Debug, serde::Serialize)]
pub struct CertStatusView {
    pub domain: String,
    pub not_after: Option<SystemTime>,
    pub state: String,
}
