use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use crate::cert_manager::state::compute_state;
use crate::cert_manager::store::CertStore;
use crate::cert_manager::{scheduler::Scheduler, state::CertState};
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
        // Desired cert IDs.
        let desired: BTreeSet<String> = desired_cert_ids_from_config(config);
        if desired.is_empty() {
            debug!("cert_manager: no desired ACME domains found");
            return;
        }

        // Actual (blocking store call for now, but might switch to async later)
        let store = self.store.clone();
        let actual_list = match tokio::task::spawn_blocking(move || store.list()).await {
            Ok(v) => v,
            Err(e) => {
                error!(error = %e, "cert_manager: store.list() join error");
                return;
            }
        };

        // Build a meta map: id -> meta
        let actual: HashMap<String, crate::cert_manager::store::CertificateMeta> =
            actual_list.into_iter().collect();

        // Reconcile desired ids
        for cert_id in desired.iter().cloned() {
            let state = compute_state(&cert_id, actual.get(&cert_id), &self.scheduler);
            if let Err(e) = self.step(cert_id, state).await {
                warn!(error = %e, "cert_manager: reconcile step failed");
            }
        }

        // Cleanup (skip for now until basic implementation is completed).
        // for id in actual.keys() {
        //     if !desired.contains(id) { ... }
        // }
    }

    use tracing::{debug, info, warn};

    async fn step(&self, cert_id: String, state: CertState) -> anyhow::Result<()> {
        match state {
            CertState::Absent => {
                info!(%cert_id, "cert_manager: cert absent; initiate order");
                // initiate order (create ACME order state, kick off Ordering)
            }

            CertState::Renewing => {
                info!(%cert_id, "cert_manager: cert renewing; initiate order");
                // initiate order (same flow as Absent)
            }

            CertState::Valid => {
                debug!(%cert_id, "cert_manager: cert valid; no-op");
            }

            // In-flight ACME states (no generic Pending)
            CertState::Ordering => {
                debug!(%cert_id, "cert_manager: ordering; advance state machine");
                // advance: place/refresh order, fetch authorizations
                // next: ChallengeInit
            }

            CertState::ChallengeInit => {
                debug!(%cert_id, "cert_manager: challenge init; prepare challenge");
                // provision challenge material (eg HTTP-01 route registration or DNS record plan)
                // next: Challenging
            }

            CertState::Challenging => {
                debug!(%cert_id, "cert_manager: challenging; poll/validate challenge");
                // trigger validation and poll authz status until valid/invalid
                // next: Finalizing (or Failed)
            }

            CertState::Finalizing => {
                debug!(%cert_id, "cert_manager: finalizing; finalize order and store cert");
                // finalize order, download cert chain, store via CertStore::put()
                // next: Valid (or Failed)
            }

            CertState::Failed => {
                warn!(%cert_id, "cert_manager: failed; backoff and retry later");
                // v1: rely on tick interval/backoff policy
            }
        }

        Ok(())
    }
}

/// Extract desired cert IDs from config.
/// This must be deterministic and must not invent domains.
fn desired_cert_ids_from_config(config: &RuntimeConfig) -> BTreeSet<String> {
    let mut out = BTreeSet::new();

    for l in &config.listeners {
        let Some(tls) = &l.tls else {
            continue;
        };
        let Some(acme) = &tls.acme_options else {
            continue;
        };

        for d in acme.domains.iter() {
            let d = d.trim().to_ascii_lowercase();
            if !d.is_empty() {
                out.insert(d);
            }
        }
    }

    out
}
