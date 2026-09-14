use crate::acme_client::AcmeClient;
use crate::admin::CertView;
use crate::challenge::Http01Registry;
use crate::error::CertManagerError;
use crate::sni_registry::{SniMap, SniRegistry};
use crate::{
    cert_store::CertStore, order_store::OrderStore, reconcile::Reconciler,
    renewal_policy::RenewalPolicy,
};
use arc_swap::ArcSwap;
use arc_swap::ArcSwapOption;
use pingora_rustls::sign;
use snakeway_conf::types::RuntimeConfig;
use snakeway_conf::types::{AcmeServerConfig, TlsAutomationConfig};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime};

pub struct CertManager {
    acme_client: OnceLock<Arc<AcmeClient>>,
    http01: Arc<Http01Registry>,
    cert_store: Arc<dyn CertStore>,
    order_store: Arc<dyn OrderStore>,
    renewal_policy: RenewalPolicy,
    config: Arc<ArcSwap<RuntimeConfig>>,
    tls_sni_map: ArcSwapOption<SniRegistry>,
}

impl CertManager {
    pub fn new(
        cert_store: Arc<dyn CertStore>,
        order_store: Arc<dyn OrderStore>,
        config: Arc<RuntimeConfig>,
        certificates_config: &TlsAutomationConfig,
    ) -> Self {
        Self {
            acme_client: Default::default(),
            http01: Arc::new(Http01Registry::default()),
            cert_store,
            order_store,
            renewal_policy: RenewalPolicy::new(certificates_config.renew_within_days),
            config: Arc::new(ArcSwap::from(config)),
            tls_sni_map: ArcSwapOption::from(None),
        }
    }

    pub async fn initialize(&self, cfg: &AcmeServerConfig) -> Result<(), CertManagerError> {
        let client = AcmeClient::load_or_create(
            cfg.directory_url.clone(),
            cfg.data_dir.clone(),
            cfg.contact_email.clone(),
            &cfg.ca_file,
        )
        .await
        .map_err(|e| CertManagerError::CannotCreateAcmeClient(e.to_string()))?;

        self.acme_client
            .set(Arc::new(client))
            .map_err(|_| CertManagerError::AlreadyInitialized)?;
        Ok(())
    }

    pub async fn run_reconciliation(self: Arc<Self>) {
        let mut reconciler = Reconciler::new(self.clone());
        reconciler.run().await;
    }

    pub fn reload(&self, new_config: Arc<RuntimeConfig>) {
        self.config.store(new_config);
    }

    pub(crate) fn load_certified_key(
        &self,
        cert_id: &str,
    ) -> Result<Option<Arc<sign::CertifiedKey>>, CertManagerError> {
        let Some(stored) = self.cert_store.get(cert_id) else {
            return Ok(None);
        };

        let certs = snakeway_conf::tls::parse_cert_chain(&stored.cert_chain_pem)
            .map_err(CertManagerError::InvalidChain)?;

        let key = snakeway_conf::tls::parse_private_key(stored.expose_private_key_pem())
            .map_err(CertManagerError::InvalidPrivateKey)?;

        let certified_key =
            snakeway_conf::tls::build_certified_key(certs, key).map_err(|e| match e {
                snakeway_conf::tls::CertKeyError::KeyMismatch => CertManagerError::KeyMismatch,
                snakeway_conf::tls::CertKeyError::Other(msg) => {
                    CertManagerError::InvalidPrivateKey(msg)
                }
            })?;

        Ok(Some(Arc::new(certified_key)))
    }

    pub fn build_sni_map(
        &self,
    ) -> Result<HashMap<String, Arc<sign::CertifiedKey>>, CertManagerError> {
        let mut map = HashMap::new();

        for (cert_id, meta) in self.cert_store.list() {
            if let Some(key) = self.load_certified_key(&cert_id)? {
                for domain in meta.domains {
                    map.insert(domain, key.clone());
                }
            }
        }

        Ok(map)
    }

    pub fn attach_tls_sni_map(&self, registry: Arc<SniRegistry>) {
        self.tls_sni_map.store(Some(registry));
    }

    pub(crate) fn tls_sni_map(&self) -> Option<Arc<SniRegistry>> {
        self.tls_sni_map.load_full()
    }

    pub(crate) fn publish_sni_map(&self, new_map: SniMap) {
        if let Some(registry) = self.tls_sni_map() {
            registry.publish(new_map);
        } else {
            tracing::warn!("acme: tls sni registry not attached; cannot publish");
        }
    }

    pub(crate) fn cert_store(&self) -> Arc<dyn CertStore> {
        self.cert_store.clone()
    }

    pub(crate) fn config(&self) -> Arc<ArcSwap<RuntimeConfig>> {
        self.config.clone()
    }

    pub(crate) fn renewal_policy(&self) -> &RenewalPolicy {
        &self.renewal_policy
    }

    pub(crate) fn order_store(&self) -> Arc<dyn OrderStore> {
        self.order_store.clone()
    }

    pub(crate) fn acme_client(&self) -> Result<Arc<AcmeClient>, CertManagerError> {
        self.acme_client
            .get()
            .cloned()
            .ok_or(CertManagerError::AcmeNotInitialized)
    }

    pub fn http01(&self) -> Arc<Http01Registry> {
        self.http01.clone()
    }
}

/// Admin API
impl CertManager {
    pub fn snapshot(&self) -> Vec<CertView> {
        let now = SystemTime::now();

        self.cert_store
            .list()
            .into_iter()
            .map(|(id, meta)| {
                let expires_in = meta
                    .not_after
                    .duration_since(now)
                    .unwrap_or(Duration::ZERO)
                    .as_secs() as i64;

                let state = if meta.not_after <= now {
                    "Expired"
                } else {
                    "Valid"
                };

                CertView {
                    id,
                    domains: meta.domains,
                    issued_at: meta.issued_at,
                    not_after: meta.not_after,
                    expires_in_seconds: expires_in,
                    state: state.to_string(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cert_store::{CertificateMeta, MemoryCertStore, StoredCertificate};
    use crate::order_store::OrderStore;
    use std::time::SystemTime;

    struct StubOrderStore;

    impl OrderStore for StubOrderStore {
        fn get(&self, _id: &str) -> std::io::Result<Option<crate::order_store::OrderState>> {
            Ok(None)
        }
        fn put(&self, _state: &crate::order_store::OrderState) -> std::io::Result<()> {
            Ok(())
        }
        fn delete(&self, _id: &str) -> std::io::Result<()> {
            Ok(())
        }
        fn list(&self) -> std::io::Result<Vec<crate::order_store::OrderState>> {
            Ok(vec![])
        }
    }

    fn make_cert_manager(store: MemoryCertStore) -> CertManager {
        let validated = snakeway_conf::load_config_from_specs(
            &snakeway_conf::types::ServerSpec::default(),
            vec![],
            vec![],
        )
        .expect("fixture config");

        CertManager {
            acme_client: OnceLock::new(),
            http01: Arc::new(Http01Registry::default()),
            cert_store: Arc::new(store),
            order_store: Arc::new(StubOrderStore),
            renewal_policy: RenewalPolicy::new(30),
            config: Arc::new(ArcSwap::from_pointee(validated.config)),
            tls_sni_map: ArcSwapOption::from(None),
        }
    }

    fn generate_stored_cert() -> StoredCertificate {
        let cert = rcgen::generate_simple_self_signed(vec!["test.example".into()])
            .expect("failed to generate cert");
        StoredCertificate::new(
            cert.signing_key.serialize_pem().into_bytes(),
            cert.cert.pem().into_bytes(),
            CertificateMeta {
                domains: vec!["test.example".to_string()],
                not_after: SystemTime::now() + std::time::Duration::from_secs(86400),
                issued_at: SystemTime::now(),
            },
        )
    }

    #[test]
    fn load_certified_key_valid_cert() {
        // Arrange
        let store = MemoryCertStore::default();
        store
            .put("test-cert".to_string(), generate_stored_cert())
            .unwrap();
        let manager = make_cert_manager(store);

        // Act
        let result = manager.load_certified_key("test-cert");

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn load_certified_key_missing_cert() {
        // Arrange
        let store = MemoryCertStore::default();
        let manager = make_cert_manager(store);

        // Act
        let result = manager.load_certified_key("nonexistent");

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn load_certified_key_empty_chain() {
        // Arrange
        let store = MemoryCertStore::default();
        let cert = rcgen::generate_simple_self_signed(vec!["test.example".into()])
            .expect("failed to generate cert");
        let stored = StoredCertificate::new(
            cert.signing_key.serialize_pem().into_bytes(),
            Vec::new(),
            CertificateMeta {
                domains: vec!["test.example".to_string()],
                not_after: SystemTime::now() + std::time::Duration::from_secs(86400),
                issued_at: SystemTime::now(),
            },
        );
        store.put("empty-chain".to_string(), stored).unwrap();
        let manager = make_cert_manager(store);

        // Act
        let result = manager.load_certified_key("empty-chain");

        // Assert
        assert!(
            matches!(result, Err(CertManagerError::InvalidChain(ref msg)) if msg.contains("no certificates")),
            "expected InvalidChain with 'no certificates', got: {result:?}"
        );
    }

    #[test]
    fn load_certified_key_mismatched_key() {
        // Arrange
        let store = MemoryCertStore::default();
        let cert1 = rcgen::generate_simple_self_signed(vec!["first.example".into()])
            .expect("failed to generate first cert");
        let cert2 = rcgen::generate_simple_self_signed(vec!["second.example".into()])
            .expect("failed to generate second cert");
        let stored = StoredCertificate::new(
            cert2.signing_key.serialize_pem().into_bytes(),
            cert1.cert.pem().into_bytes(),
            CertificateMeta {
                domains: vec!["first.example".to_string()],
                not_after: SystemTime::now() + std::time::Duration::from_secs(86400),
                issued_at: SystemTime::now(),
            },
        );
        store.put("mismatched".to_string(), stored).unwrap();
        let manager = make_cert_manager(store);

        // Act
        let result = manager.load_certified_key("mismatched");

        // Assert
        assert!(matches!(result, Err(CertManagerError::KeyMismatch)));
    }
}
