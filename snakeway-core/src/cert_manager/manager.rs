use arc_swap::ArcSwap;
use openssl::pkey::{PKey, Private};
use openssl::x509::X509;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::task::JoinHandle;

use crate::cert_manager::acme_client::AcmeClient;
use crate::cert_manager::challenge::Http01Registry;
use crate::cert_manager::error::CertManagerError;
use crate::cert_manager::{
    ParsedCert, cert_store::CertStore, order_store::OrderStore, reconcile::Reconciler,
    renewal_policy::RenewalPolicy,
};
use crate::conf::RuntimeConfig;
use crate::conf::types::{AcmeServerConfig, CertificatesConfig};

pub struct CertManager {
    acme_client: OnceLock<Arc<AcmeClient>>,
    http01: Arc<Http01Registry>,
    cert_store: Arc<dyn CertStore>,
    order_store: Arc<dyn OrderStore>,
    renewal_policy: RenewalPolicy,

    worker: Mutex<Option<JoinHandle<()>>>,
    config: Arc<ArcSwap<RuntimeConfig>>,
}

impl CertManager {
    pub fn new(
        cert_store: Arc<dyn CertStore>,
        order_store: Arc<dyn OrderStore>,
        config: Arc<RuntimeConfig>,
        certificates_config: &CertificatesConfig,
    ) -> Self {
        Self {
            acme_client: Default::default(),
            http01: Arc::new(Http01Registry::default()),
            cert_store,
            order_store,
            renewal_policy: RenewalPolicy::new(certificates_config.renew_within_days),
            worker: Mutex::new(None),
            config: Arc::new(ArcSwap::from(config)),
        }
    }

    pub async fn initialize(&self, cfg: &AcmeServerConfig) -> Result<(), CertManagerError> {
        let client = AcmeClient::load_or_create(
            cfg.directory_url.clone(),
            cfg.data_dir.clone(),
            cfg.contact_email.clone(),
        )
        .await
        .map_err(|e| CertManagerError::CannotCreateAcmeClient(e.to_string()))?;

        self.acme_client
            .set(Arc::new(client))
            .map_err(|_| CertManagerError::AlreadyInitialized)?;
        Ok(())
    }

    pub fn run_reconciliation(self: Arc<Self>) -> impl Future<Output = ()> {
        async move {
            let mut reconciler = Reconciler::new(self.clone());
            reconciler.run().await;
        }
    }

    pub fn reload(&self, new_config: Arc<RuntimeConfig>) {
        self.config.store(new_config);
    }

    pub async fn shutdown(&self) {
        let mut guard = self.worker.lock().unwrap();

        if let Some(handle) = guard.take() {
            handle.abort();
        }
    }

    pub fn load_parsed_cert(&self, cert_id: &str) -> Result<Option<ParsedCert>, CertManagerError> {
        let Some(stored) = self.cert_store.get(cert_id) else {
            return Ok(None);
        };

        let mut chain = X509::stack_from_pem(&stored.cert_chain_pem)
            .map_err(|e| CertManagerError::InvalidChain(e.to_string()))?;

        if chain.is_empty() {
            return Err(CertManagerError::EmptyChain);
        }

        let leaf = chain.remove(0);

        let key = PKey::<Private>::private_key_from_pem(&stored.private_key_pem)
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

    pub fn cert_store(&self) -> Arc<dyn CertStore> {
        self.cert_store.clone()
    }

    pub fn config(&self) -> Arc<ArcSwap<RuntimeConfig>> {
        self.config.clone()
    }

    pub fn renewal_policy(&self) -> &RenewalPolicy {
        &self.renewal_policy
    }

    pub fn order_store(&self) -> Arc<dyn OrderStore> {
        self.order_store.clone()
    }

    pub fn acme_client(&self) -> Result<Arc<AcmeClient>, CertManagerError> {
        self.acme_client
            .get()
            .cloned()
            .ok_or(CertManagerError::AcmeNotInitialized)
    }

    pub fn http01(&self) -> Arc<Http01Registry> {
        self.http01.clone()
    }
}
