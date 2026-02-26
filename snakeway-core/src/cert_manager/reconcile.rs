use crate::cert_manager::cert_store::{CertificateMeta, StoredCertificate};
use crate::cert_manager::order_store::OrderState;
use crate::cert_manager::state::{CertState, compute_state};
use crate::cert_manager::{CertManager, OrderStatus};
use crate::conf::RuntimeConfig;
use crate::conf::types::{CertificateChallengeConfig, CertificateConfig};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

#[derive(Debug, Error)]
pub enum ReconcilerError {
    #[error("acme client not available: {0}")]
    CannotGetAcmeClient(String),

    #[error("acme error: {0}")]
    Acme(String),

    #[error("order store error: {0}")]
    OrderStore(String),

    #[error("cert store error: {0}")]
    CertStore(String),

    #[error("no pending authorization found")]
    NoPendingAuthorization,

    #[error("no http-01 challenge found")]
    NoHttp01Challenge,

    #[error("unexpected order status: {0:?}")]
    UnexpectedOrderStatus(instant_acme::OrderStatus),
}

pub struct Reconciler {
    cert_manager: Arc<CertManager>,
}

impl Reconciler {
    pub fn new(cert_manager: Arc<CertManager>) -> Self {
        Self { cert_manager }
    }

    pub async fn run(&mut self) {
        loop {
            let config = self.cert_manager.config().load();

            if let Err(e) = self.tick(&config).await {
                error!(error = %e, "cert_manager: reconcile tick failed");
            }

            sleep(self.cert_manager.renewal_policy().reconcile_interval).await;
        }
    }

    async fn tick(&self, config: &RuntimeConfig) -> Result<(), ReconcilerError> {
        let desired = desired_certificates_from_config(config);

        if desired.is_empty() {
            debug!("cert_manager: no desired ACME domains found");
            return Ok(());
        }

        let cert_store = self.cert_manager.cert_store();
        let actual_list = tokio::task::spawn_blocking(move || cert_store.list())
            .await
            .map_err(|e| ReconcilerError::CertStore(format!("join error: {e}")))?;

        let actual: HashMap<String, CertificateMeta> = actual_list.into_iter().collect();

        for (cert_id, desired_cert) in desired {
            let order_store = self.cert_manager.order_store();
            let id = cert_id.clone();

            let order_state = tokio::task::spawn_blocking(move || order_store.get(&id))
                .await
                .map_err(|e| ReconcilerError::OrderStore(format!("join error: {e}")))?
                .map_err(|e| ReconcilerError::OrderStore(format!("io error: {e}")))?;

            let state = compute_state(
                &cert_id,
                actual.get(&cert_id),
                order_state.as_ref(),
                &self.cert_manager.renewal_policy(),
            );

            if let Err(e) = self
                .step(cert_id.clone(), desired_cert, state, order_state)
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
        desired: DesiredCertificate,
        state: CertState,
        order_state: Option<OrderState>,
    ) -> Result<(), ReconcilerError> {
        match state {
            CertState::Absent | CertState::Renewing => {
                self.step_new_order(cert_id, desired).await?;
            }
            CertState::Ordering => {
                self.step_select_http01(cert_id, order_state).await?;
            }
            CertState::ChallengeInit => {
                self.step_set_ready(cert_id, order_state).await?;
            }
            CertState::Challenging => {
                self.step_poll_ready(cert_id, order_state).await?;
            }
            CertState::Finalizing => {
                self.step_finalize_and_store(cert_id, desired, order_state)
                    .await?;
            }
            CertState::Valid => {}
            CertState::Failed => {}
        }
        Ok(())
    }

    async fn step_new_order(
        &self,
        cert_id: String,
        desired: DesiredCertificate,
    ) -> Result<(), ReconcilerError> {
        use instant_acme::{Identifier, NewOrder};

        info!(%cert_id, "creating new ACME order");

        let identifiers: Vec<Identifier> = desired
            .domains
            .iter()
            .map(|d| Identifier::Dns(d.clone()))
            .collect();

        let client = self
            .cert_manager
            .acme_client()
            .map_err(|e| ReconcilerError::CannotGetAcmeClient(e.to_string()))?;

        let mut order = client
            .account
            .new_order(&NewOrder::new(&identifiers))
            .await
            .map_err(|e| ReconcilerError::Acme(e.to_string()))?;

        let order_url = order.url().to_string();
        let order_state = order.state();

        if order_state.status != instant_acme::OrderStatus::Pending {
            return Err(ReconcilerError::UnexpectedOrderStatus(order_state.status));
        }

        let new_state = OrderState {
            cert_id: cert_id.clone(),
            domains: desired.domains,
            challenge: desired.challenge,
            status: OrderStatus::Ordering,
            order_url,
            authorization_urls: order_state
                .authorizations
                .iter()
                .map(|a| a.url.clone())
                .collect(),
            challenge_url: None,
            challenge_token: None,
            challenge_key_authorization: None,
            failure_count: 0,
            last_error: None,
            updated_at: std::time::SystemTime::now(),
        };

        let store = self.cert_manager.order_store();
        tokio::task::spawn_blocking(move || store.put(&new_state))
            .await
            .map_err(|e| ReconcilerError::OrderStore(format!("join error: {e}")))?
            .map_err(|e| ReconcilerError::OrderStore(format!("io error: {e}")))?;

        Ok(())
    }

    async fn step_select_http01(
        &self,
        cert_id: String,
        order_state: Option<OrderState>,
    ) -> Result<(), ReconcilerError> {
        use instant_acme::{AuthorizationStatus, ChallengeType};

        let order_state =
            order_state.ok_or_else(|| ReconcilerError::OrderStore("missing order state".into()))?;

        let client = self
            .cert_manager
            .acme_client()
            .map_err(|e| ReconcilerError::CannotGetAcmeClient(e.to_string()))?;

        let mut order = client
            .account
            .order(order_state.order_url.clone())
            .await
            .map_err(|e| ReconcilerError::Acme(e.to_string()))?;

        let mut authzs = order.authorizations();

        while let Some(res) = authzs.next().await {
            let mut authz = res.map_err(|e| ReconcilerError::Acme(e.to_string()))?;

            if authz.status != AuthorizationStatus::Pending {
                continue;
            }

            let challenge = authz
                .challenge(ChallengeType::Http01)
                .ok_or(ReconcilerError::NoHttp01Challenge)?;

            let token = challenge.token.to_string();
            let key_auth = challenge.key_authorization().as_str().to_string();
            let challenge_url = challenge.url.to_string();

            self.cert_manager
                .http01()
                .put(token.clone(), key_auth.clone());

            let mut new_state = order_state.clone();
            new_state.status = OrderStatus::ChallengeInit;
            new_state.challenge_token = Some(token);
            new_state.challenge_key_authorization = Some(key_auth);
            new_state.challenge_url = Some(challenge_url);
            new_state.updated_at = std::time::SystemTime::now();

            let store = self.cert_manager.order_store();
            tokio::task::spawn_blocking(move || store.put(&new_state))
                .await
                .map_err(|e| ReconcilerError::OrderStore(format!("join error: {e}")))?
                .map_err(|e| ReconcilerError::OrderStore(format!("io error: {e}")))?;

            return Ok(());
        }

        Err(ReconcilerError::NoPendingAuthorization)
    }

    async fn step_set_ready(
        &self,
        cert_id: String,
        order_state: Option<OrderState>,
    ) -> Result<(), ReconcilerError> {
        let order_state =
            order_state.ok_or_else(|| ReconcilerError::OrderStore("missing order state".into()))?;

        let client = self
            .cert_manager
            .acme_client()
            .map_err(|e| ReconcilerError::CannotGetAcmeClient(e.to_string()))?;

        let mut order = client
            .account
            .order(order_state.order_url.clone())
            .await
            .map_err(|e| ReconcilerError::Acme(e.to_string()))?;

        let mut authzs = order.authorizations();

        while let Some(res) = authzs.next().await {
            let mut authz = res.map_err(|e| ReconcilerError::Acme(e.to_string()))?;

            if let Some(mut challenge) = authz.challenge(instant_acme::ChallengeType::Http01) {
                challenge
                    .set_ready()
                    .await
                    .map_err(|e| ReconcilerError::Acme(e.to_string()))?;

                let mut new_state = order_state.clone();
                new_state.status = OrderStatus::Challenging;
                new_state.updated_at = std::time::SystemTime::now();

                let store = self.cert_manager.order_store();
                tokio::task::spawn_blocking(move || store.put(&new_state))
                    .await
                    .map_err(|e| ReconcilerError::OrderStore(format!("join error: {e}")))?
                    .map_err(|e| ReconcilerError::OrderStore(format!("io error: {e}")))?;

                return Ok(());
            }
        }

        Err(ReconcilerError::NoHttp01Challenge)
    }

    async fn step_poll_ready(
        &self,
        cert_id: String,
        order_state: Option<OrderState>,
    ) -> Result<(), ReconcilerError> {
        use instant_acme::{OrderStatus as AcmeOrderStatus, RetryPolicy};

        let order_state =
            order_state.ok_or_else(|| ReconcilerError::OrderStore("missing order state".into()))?;

        let client = self
            .cert_manager
            .acme_client()
            .map_err(|e| ReconcilerError::CannotGetAcmeClient(e.to_string()))?;

        let mut order = client
            .account
            .order(order_state.order_url.clone())
            .await
            .map_err(|e| ReconcilerError::Acme(e.to_string()))?;

        let status = order
            .poll_ready(&RetryPolicy::default())
            .await
            .map_err(|e| ReconcilerError::Acme(e.to_string()))?;

        if status != AcmeOrderStatus::Ready {
            return Ok(());
        }

        let mut new_state = order_state.clone();
        new_state.status = OrderStatus::Finalizing;
        new_state.updated_at = std::time::SystemTime::now();

        let store = self.cert_manager.order_store();
        tokio::task::spawn_blocking(move || store.put(&new_state))
            .await
            .map_err(|e| ReconcilerError::OrderStore(format!("join error: {e}")))?
            .map_err(|e| ReconcilerError::OrderStore(format!("io error: {e}")))?;

        Ok(())
    }

    async fn step_finalize_and_store(
        &self,
        cert_id: String,
        desired: DesiredCertificate,
        order_state: Option<OrderState>,
    ) -> Result<(), ReconcilerError> {
        use instant_acme::RetryPolicy;

        let order_state =
            order_state.ok_or_else(|| ReconcilerError::OrderStore("missing order state".into()))?;

        let client = self
            .cert_manager
            .acme_client()
            .map_err(|e| ReconcilerError::CannotGetAcmeClient(e.to_string()))?;

        let mut order = client
            .account
            .order(order_state.order_url.clone())
            .await
            .map_err(|e| ReconcilerError::Acme(e.to_string()))?;

        let private_key_pem = order
            .finalize()
            .await
            .map_err(|e| ReconcilerError::Acme(e.to_string()))?;

        let cert_chain_pem = order
            .poll_certificate(&RetryPolicy::default())
            .await
            .map_err(|e| ReconcilerError::Acme(e.to_string()))?;

        let meta = CertificateMeta {
            domains: desired.domains,
            not_after: std::time::SystemTime::now(),
            issued_at: std::time::SystemTime::now(),
        };

        let stored = StoredCertificate {
            private_key_pem: private_key_pem.into_bytes(),
            cert_chain_pem: cert_chain_pem.into_bytes(),
            meta,
        };

        let cert_store = self.cert_manager.cert_store();
        let id = cert_id.clone();
        tokio::task::spawn_blocking(move || cert_store.put(id, stored))
            .await
            .map_err(|e| ReconcilerError::CertStore(format!("join error: {e}")))?
            .map_err(|e| ReconcilerError::CertStore(format!("io error: {e}")))?;

        if let Some(token) = order_state.challenge_token {
            self.cert_manager.http01().remove(&token);
        }

        let order_store = self.cert_manager.order_store();
        let id = cert_id.clone();
        tokio::task::spawn_blocking(move || order_store.delete(&id))
            .await
            .map_err(|e| ReconcilerError::OrderStore(format!("join error: {e}")))?
            .map_err(|e| ReconcilerError::OrderStore(format!("io error: {e}")))?;

        info!(%cert_id, "certificate successfully issued and stored");

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DesiredCertificate {
    pub domains: Vec<String>,
    pub challenge: CertificateChallengeConfig,
}

fn desired_certificates_from_config(
    config: &RuntimeConfig,
) -> BTreeMap<String, DesiredCertificate> {
    let mut out = BTreeMap::new();

    for l in &config.listeners {
        let Some(certificate_config) = &l.certificates else {
            continue;
        };

        if let CertificateConfig::Acme { domains, challenge } = certificate_config {
            let cert_id = compute_cert_id(domains, challenge);
            out.insert(
                cert_id,
                DesiredCertificate {
                    domains: domains.clone(),
                    challenge: challenge.clone(),
                },
            );
        }
    }

    out
}

fn compute_cert_id(domains: &[String], challenge: &CertificateChallengeConfig) -> String {
    let mut hasher = Sha256::new();

    for d in domains {
        hasher.update(d.as_bytes());
        hasher.update(b"\0");
    }

    hasher.update(format!("{challenge:?}").as_bytes());

    let digest = hasher.finalize();
    format!("{:x}", digest)[..32].to_string()
}
