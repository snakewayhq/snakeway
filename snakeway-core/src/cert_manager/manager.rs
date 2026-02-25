use arc_swap::ArcSwap;
use openssl::pkey::{PKey, Private};
use openssl::x509::X509;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

use crate::cert_manager::error::CertManagerError;
use crate::cert_manager::{
    ParsedCert, reconcile::Reconciler, renewal_policy::RenewalPolicy, store::CertStore,
};
use crate::conf::RuntimeConfig;

pub struct CertManager {
    store: Arc<dyn CertStore>,
    scheduler: RenewalPolicy,

    // Worker lifecycle (interior mutable because manager lives behind Arc)
    worker: Mutex<Option<JoinHandle<()>>>,

    // Reloadable config.
    config: Arc<ArcSwap<RuntimeConfig>>,
}

impl CertManager {
    pub fn new(
        store: Arc<dyn CertStore>,
        renew_within_days: u64,
        config: Arc<RuntimeConfig>,
    ) -> Self {
        Self {
            store,
            scheduler: RenewalPolicy::new(renew_within_days),
            worker: Mutex::new(None),
            config: Arc::new(ArcSwap::from(config)),
        }
    }

    /// Start background reconciliation loop.
    /// Safe to call multiple times — will only start once.
    pub fn start(self: &Arc<Self>, runtime: &tokio::runtime::Runtime) {
        let mut guard = self.worker.lock().unwrap();

        if guard.is_some() {
            // Already started
            return;
        }

        let store = self.store.clone();
        let scheduler = self.scheduler.clone();
        let config = self.config.clone();

        let handle = runtime.spawn(async move {
            let mut reconciler = Reconciler::new(store, scheduler, config);
            reconciler.run().await;
        });

        *guard = Some(handle);
    }

    /// Called during hot reload.
    ///
    /// Does not restart worker. Intended to update shared config.
    pub fn reload(&self, new_config: Arc<RuntimeConfig>) {
        self.config.store(new_config);
    }

    /// Graceful shutdown.
    pub async fn shutdown(&self) {
        let mut guard = self.worker.lock().unwrap();

        if let Some(handle) = guard.take() {
            handle.abort();
        }
    }

    pub fn store(&self) -> Arc<dyn CertStore> {
        self.store.clone()
    }

    /// Load and parse a certificate from the store.
    pub fn load_parsed_cert(&self, cert_id: &str) -> Result<Option<ParsedCert>, CertManagerError> {
        let Some(stored) = self.store.get(cert_id) else {
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

    /// Build SNI to ParsedCert map from store.
    pub fn build_sni_map(&self) -> Result<HashMap<String, Arc<ParsedCert>>, CertManagerError> {
        let mut map = HashMap::new();

        for (cert_id, meta) in self.store.list() {
            if let Some(parsed) = self.load_parsed_cert(&cert_id)? {
                let parsed = Arc::new(parsed);

                for domain in meta.domains {
                    map.insert(domain, parsed.clone());
                }
            }
        }

        Ok(map)
    }
}
