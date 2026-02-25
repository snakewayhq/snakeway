use arc_swap::ArcSwap;
use openssl::pkey::{PKey, Private};
use openssl::x509::X509;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

use crate::cert_manager::error::CertManagerError;
use crate::cert_manager::{
    ParsedCert, cert_store::CertStore, order_store::OrderStore, reconcile::Reconciler,
    renewal_policy::RenewalPolicy,
};
use crate::conf::RuntimeConfig;

pub struct CertManager {
    cert_store: Arc<dyn CertStore>,
    order_store: Arc<dyn OrderStore>,
    scheduler: RenewalPolicy,

    worker: Mutex<Option<JoinHandle<()>>>,
    config: Arc<ArcSwap<RuntimeConfig>>,
}

impl CertManager {
    pub fn new(
        cert_store: Arc<dyn CertStore>,
        order_store: Arc<dyn OrderStore>,
        renew_within_days: u64,
        config: Arc<RuntimeConfig>,
    ) -> Self {
        Self {
            cert_store,
            order_store,
            scheduler: RenewalPolicy::new(renew_within_days),
            worker: Mutex::new(None),
            config: Arc::new(ArcSwap::from(config)),
        }
    }

    pub fn start(self: &Arc<Self>, runtime: &tokio::runtime::Runtime) {
        let mut guard = self.worker.lock().unwrap();

        if guard.is_some() {
            return;
        }

        let cert_store = self.cert_store.clone();
        let order_store = self.order_store.clone();
        let scheduler = self.scheduler.clone();
        let config = self.config.clone();

        let handle = runtime.spawn(async move {
            let mut reconciler = Reconciler::new(order_store, cert_store, scheduler, config);
            reconciler.run().await;
        });

        *guard = Some(handle);
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

    pub fn cert_store(&self) -> Arc<dyn CertStore> {
        self.cert_store.clone()
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
}
