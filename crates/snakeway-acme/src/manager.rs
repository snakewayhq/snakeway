use crate::acme_client::AcmeClient;
use crate::admin::CertView;
use crate::challenge::Http01Registry;
use crate::error::CertManagerError;
use crate::sni_registry::{SniMap, SniRegistry};
use crate::{
    ParsedCert, cert_store::CertStore, order_store::OrderStore, reconcile::Reconciler,
    renewal_policy::RenewalPolicy,
};
use arc_swap::ArcSwap;
use arc_swap::ArcSwapOption;
use openssl::pkey::{PKey, Private};
use openssl::x509::X509;
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

    pub(crate) fn load_parsed_cert(
        &self,
        cert_id: &str,
    ) -> Result<Option<ParsedCert>, CertManagerError> {
        let Some(stored) = self.cert_store.get(cert_id) else {
            return Ok(None);
        };

        let mut chain = X509::stack_from_pem(&stored.cert_chain_pem)
            .map_err(|e| CertManagerError::InvalidChain(e.to_string()))?;

        if chain.is_empty() {
            return Err(CertManagerError::EmptyChain);
        }

        let leaf = chain.remove(0);

        let key = PKey::<Private>::private_key_from_pem(stored.expose_private_key_pem())
            .map_err(|e| CertManagerError::InvalidPrivateKey(e.to_string()))?;

        let public_key = leaf
            .public_key()
            .map_err(|e| CertManagerError::InvalidChain(e.to_string()))?;

        if !public_key.public_eq(&key) {
            return Err(CertManagerError::KeyMismatch);
        }

        Ok(Some(ParsedCert { leaf, chain, key }))
    }

    pub fn build_sni_map(&self) -> Result<HashMap<String, Arc<ParsedCert>>, CertManagerError> {
        let mut map = HashMap::new();

        for (cert_id, meta) in self.cert_store.list() {
            if let Some(parsed) = self.load_parsed_cert(&cert_id)? {
                let parsed = Arc::new(parsed);

                for domain in meta.domains {
                    map.insert(domain, parsed.clone());
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
