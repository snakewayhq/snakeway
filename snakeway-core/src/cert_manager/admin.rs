use serde::Serialize;
use std::time::SystemTime;

#[derive(Serialize)]
pub struct CertView {
    pub id: String,
    pub domains: Vec<String>,
    pub issued_at: SystemTime,
    pub not_after: SystemTime,
    pub expires_in_seconds: i64,
    pub state: String,
}
