//! Control-plane subsystem responsible for:
//! 1. ACME orchestration
//! 2. Certificate storage
//! 3. Renewal scheduling

mod acme_client;
mod admin;
mod cert_store;
mod challenge;
mod error;
mod manager;
mod order_store;
mod parsed_cert;
mod reconcile;
mod renewal_policy;
mod sni_registry;
mod state;

pub use manager::CertManager;
pub(crate) use parsed_cert::ParsedCert;
pub use sni_registry::SniRegistry;

pub use cert_store::*;
pub use order_store::*;

pub mod prelude {
    //! The public surface consumers need to hold and construct certificate
    //! management: the `CertManager`, the SNI registry, and the cert/order
    //! store traits with their filesystem and in-memory implementations.
    pub use crate::{
        CertManager, CertStore, FilesystemCertStore, FilesystemOrderStore, MemoryCertStore,
        OrderStore, SniRegistry,
    };
}
