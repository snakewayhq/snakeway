use serde::Serialize;
use std::time::SystemTime;

#[derive(Serialize)]
pub struct CertView {
    pub(crate) id: String,
    pub(crate) domains: Vec<String>,
    pub(crate) issued_at: SystemTime,
    pub(crate) not_after: SystemTime,
    pub(crate) expires_in_seconds: i64,
    pub(crate) state: String,
}
