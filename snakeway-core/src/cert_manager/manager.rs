use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::cert_manager::{reconcile::Reconciler, renewal_policy::RenewalPolicy, store::CertStore};
use crate::conf::RuntimeConfig;

pub struct CertManager {
    store: Arc<dyn CertStore>,
    scheduler: RenewalPolicy,

    // Worker lifecycle
    worker: Option<JoinHandle<()>>,
}

impl CertManager {
    pub fn new(store: Arc<dyn CertStore>, renew_within_days: u64) -> Self {
        Self {
            store,
            scheduler: RenewalPolicy::new(renew_within_days),
            worker: None,
        }
    }

    /// Start background reconciliation loop.
    pub fn start(&mut self, runtime: &tokio::runtime::Runtime, config: Arc<RuntimeConfig>) {
        let store = self.store.clone();
        let scheduler = self.scheduler.clone();

        self.worker = Some(runtime.spawn(async move {
            let mut reconciler = Reconciler::new(store, scheduler);
            reconciler.run(config).await;
        }));
    }

    /// Called during hot reload.
    ///
    /// This must NOT restart the whole subsystem.
    /// It simply provides new desired state.
    pub fn reload(&self, new_config: Arc<RuntimeConfig>) {
        // In v1 this may update an Arc<AtomicConfig> that Reconciler reads.
        // Avoid killing the worker unless absolutely necessary.
        //
        // Implementation detail handled inside Reconciler.
        todo!("wire config update into reconciler");
    }

    /// Graceful shutdown.
    pub async fn shutdown(&mut self) {
        if let Some(handle) = self.worker.take() {
            handle.abort();
        }
    }

    pub fn store(&self) -> Arc<dyn CertStore> {
        self.store.clone()
    }
}
