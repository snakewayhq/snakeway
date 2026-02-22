//! Control-plane subsystem responsible for:
//! 1. ACME orchestration
//! 2. Certificate storage
//! 3. Renewal scheduling

mod acme_client;
mod challenge;
mod error;
mod manager;
mod parsed_cert;
mod reconcile;
mod renewal_policy;
mod state;
mod store;

pub use manager::CertManager;
pub use parsed_cert::ParsedCert;
pub use store::*;
