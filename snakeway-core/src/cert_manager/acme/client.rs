use crate::cert_manager::acme::errors::AcmeError;
use crate::cert_manager::store::store_trait::StoredCertificate;

#[async_trait::async_trait]
pub trait AcmeClient: Send + Sync {
    async fn issue_certificate(&self, domains: &[String]) -> Result<StoredCertificate, AcmeError>;
}
