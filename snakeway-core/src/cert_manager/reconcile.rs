use crate::cert_manager::cert_store::{CertStore, CertificateMeta};
use crate::cert_manager::order_store::{OrderState, OrderStore};
use crate::cert_manager::state::compute_state;
use crate::cert_manager::{renewal_policy::RenewalPolicy, state::CertState};
use crate::conf::RuntimeConfig;

use arc_swap::ArcSwap;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

pub struct Reconciler {
    order_store: Arc<dyn OrderStore>,
    cert_store: Arc<dyn CertStore>,
    renewal_policy: RenewalPolicy,
    config: Arc<ArcSwap<RuntimeConfig>>,
}

impl Reconciler {
    pub fn new(
        order_store: Arc<dyn OrderStore>,
        cert_store: Arc<dyn CertStore>,
        renewal_policy: RenewalPolicy,
        config: Arc<ArcSwap<RuntimeConfig>>,
    ) -> Self {
        Self {
            order_store,
            cert_store,
            renewal_policy,
            config,
        }
    }

    pub async fn run(&mut self) {
        loop {
            let config = self.config.load();
            if let Err(e) = self.tick(&config).await {
                error!(error = %e, "cert_manager: reconcile tick failed");
            }
            sleep(self.renewal_policy.reconcile_interval).await;
        }
    }

    async fn tick(&self, config: &RuntimeConfig) -> anyhow::Result<()> {
        let desired: BTreeSet<String> = desired_cert_ids_from_config(config);

        if desired.is_empty() {
            debug!("cert_manager: no desired ACME domains found");
            return Ok(());
        }

        // Load certificate metadata (blocking CertStore::list() -> Vec<_>)
        let cert_store = self.cert_store.clone();
        let actual_list: Vec<(String, CertificateMeta)> =
            tokio::task::spawn_blocking(move || cert_store.list()).await?;

        let actual: HashMap<String, CertificateMeta> = actual_list.into_iter().collect();

        for cert_id in desired.into_iter() {
            // Load order state (blocking OrderStore::get() -> io::Result<Option<OrderState>>)
            let order_store = self.order_store.clone();
            let cert_id_clone = cert_id.clone();

            let order_state: Option<OrderState> =
                tokio::task::spawn_blocking(move || order_store.get(&cert_id_clone)).await??;

            let state = compute_state(
                &cert_id,
                actual.get(&cert_id),
                order_state.as_ref(),
                &self.renewal_policy,
            );

            if let Err(e) = self
                .step(
                    cert_id.clone(),
                    state,
                    order_state,
                    actual.get(&cert_id).cloned(),
                )
                .await
            {
                warn!(error = %e, "cert_manager: reconcile step failed");
            }
        }

        Ok(())
    }

    async fn step(
        &self,
        cert_id: String,
        state: CertState,
        _order_state: Option<OrderState>,
        _cert_meta: Option<CertificateMeta>,
    ) -> anyhow::Result<()> {
        match state {
            CertState::Absent => {
                info!(%cert_id, "cert_manager: cert absent; initiate order");
            }
            CertState::Renewing => {
                info!(%cert_id, "cert_manager: cert renewing; initiate order");
            }
            CertState::Valid => {
                debug!(%cert_id, "cert_manager: cert valid; no-op");
            }
            CertState::Ordering => {
                debug!(%cert_id, "cert_manager: ordering; advance state machine");
            }
            CertState::ChallengeInit => {
                debug!(%cert_id, "cert_manager: challenge init; prepare challenge");
            }
            CertState::Challenging => {
                debug!(%cert_id, "cert_manager: challenging; poll/validate challenge");
            }
            CertState::Finalizing => {
                debug!(%cert_id, "cert_manager: finalizing; finalize order and store cert");
            }
            CertState::Failed => {
                warn!(%cert_id, "cert_manager: failed; backoff active");
            }
        }

        Ok(())
    }
}

fn desired_cert_ids_from_config(config: &RuntimeConfig) -> BTreeSet<String> {
    let mut out = BTreeSet::new();

    for l in &config.listeners {
        let Some(tls) = &l.tls else { continue };
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
