use std::sync::Arc;
use tokio::time::sleep;

use crate::cert_manager::store::store_trait::CertStore;
use crate::cert_manager::{
    scheduler::Scheduler,
    state::{CertState, compute_state},
};
use crate::conf::RuntimeConfig;

pub struct Reconciler {
    store: Arc<dyn CertStore>,
    scheduler: Scheduler,
}

impl Reconciler {
    pub fn new(store: Arc<dyn CertStore>, scheduler: Scheduler) -> Self {
        Self { store, scheduler }
    }

    pub async fn run(&mut self, config: Arc<RuntimeConfig>) {
        loop {
            self.tick(&config).await;

            sleep(self.scheduler.tick_interval()).await;
        }
    }

    async fn tick(&self, config: &RuntimeConfig) {
        todo!("implement reconciliation")
        // let desired = config.desired_tls_domains();
        // let actual = self.store.list();
        //
        // for cert_id in desired {
        //     let state = compute_state(&cert_id, &actual);
        //
        //     self.step(cert_id, state).await;
        // }
    }

    async fn step(&self, cert_id: String, state: CertState) {
        match state {
            CertState::Absent => {
                // initiate order
            }
            CertState::Renewing => {
                // same flow as Absent
            }
            _ => {}
        }
    }
}
