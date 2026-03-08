use crate::conf::RuntimeConfig;
use crate::conf::types::{AcmeChallengeConfig, TlsTerminationConfig};
use crate::control_plane::acme::cert_store::{CertificateMeta, StoredCertificate};
use crate::control_plane::acme::order_store::OrderState;
use crate::control_plane::acme::state::{CertState, compute_state};
use crate::control_plane::acme::{CertManager, OrderStatus};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use x509_parser::prelude::*;

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

            let sleep_interval = if self.has_active_orders(&config).await {
                self.cert_manager.renewal_policy().order_poll_interval
            } else {
                self.cert_manager.renewal_policy().reconcile_interval
            };

            sleep(sleep_interval).await;
        }
    }

    async fn has_active_orders(&self, config: &RuntimeConfig) -> bool {
        let desired = desired_certificates_from_config(config);

        for cert_id in desired.keys() {
            let order_store = self.cert_manager.order_store();
            let id = cert_id.clone();

            let order_state = tokio::task::spawn_blocking(move || order_store.get(&id))
                .await
                .ok()
                .and_then(|r| r.ok())
                .flatten();

            if let Some(state) = order_state {
                match state.status {
                    OrderStatus::Ordering
                    | OrderStatus::ChallengeInit
                    | OrderStatus::Challenging
                    | OrderStatus::Finalizing => return true,
                    _ => {}
                }
            }
        }

        false
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
                actual.get(&cert_id),
                order_state.as_ref(),
                self.cert_manager.renewal_policy(),
            );

            if let Err(e) = self
                .step(cert_id.clone(), desired_cert, &state, order_state)
                .await
            {
                warn!(error = %e, state = %state, "cert_manager: reconcile step failed");
            }
        }

        Ok(())
    }

    async fn step(
        &self,
        cert_id: String,
        desired: DesiredCertificate,
        state: &CertState,
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
            CertState::Valid => {
                info!(%cert_id, "acme: certificate is valid");
            }
            CertState::Failed => {
                error!(%cert_id, "acme: certificate failed");
            }
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

        let new_status = match order_state.status {
            instant_acme::OrderStatus::Pending => OrderStatus::Ordering,
            instant_acme::OrderStatus::Ready => OrderStatus::Finalizing,
            other => return Err(ReconcilerError::UnexpectedOrderStatus(other)),
        };

        let new_state = OrderState {
            cert_id: cert_id.clone(),
            domains: desired.domains,
            challenge: desired.challenge,
            status: new_status,
            order_url,
            authorization_urls: order_state
                .authorizations
                .iter()
                .map(|a| a.url.clone())
                .collect(),
            challenge_tokens: Vec::new(),
            failure_count: 0,
            last_error: None,
            updated_at: std::time::SystemTime::now(),
        };

        let store = self.cert_manager.order_store();
        tokio::task::spawn_blocking(move || store.put(&new_state))
            .await
            .map_err(|e| ReconcilerError::OrderStore(format!("join error: {e}")))?
            .map_err(|e| ReconcilerError::OrderStore(format!("io error: {e}")))?;

        info!(%cert_id, "acme: order persisted; entering Ordering state");
        Ok(())
    }

    async fn step_select_http01(
        &self,
        cert_id: String,
        order_state: Option<OrderState>,
    ) -> Result<(), ReconcilerError> {
        use instant_acme::{AuthorizationStatus, ChallengeType};

        info!(%cert_id, "acme: registering http-01 challenge token(s)");

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

        // Collect a (token, keyAuthorization) pair for every pending authorization.
        // For single-domain orders this produces one entry; for SAN orders covering
        // N domains it produces N entries, one per domain that still needs validation.
        let mut challenge_tokens: Vec<(String, String)> = Vec::new();

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

            self.cert_manager
                .http01()
                .put(token.clone(), key_auth.clone());

            challenge_tokens.push((token, key_auth));
        }

        if challenge_tokens.is_empty() {
            return Err(ReconcilerError::NoPendingAuthorization);
        }

        let mut new_state = order_state.clone();
        new_state.status = OrderStatus::ChallengeInit;
        new_state.challenge_tokens = challenge_tokens;
        new_state.updated_at = std::time::SystemTime::now();

        let store = self.cert_manager.order_store();
        tokio::task::spawn_blocking(move || store.put(&new_state))
            .await
            .map_err(|e| ReconcilerError::OrderStore(format!("join error: {e}")))?
            .map_err(|e| ReconcilerError::OrderStore(format!("io error: {e}")))?;

        info!(%cert_id, "acme: all pending challenge tokens registered; entering ChallengeInit state");
        Ok(())
    }

    async fn step_set_ready(
        &self,
        cert_id: String,
        order_state: Option<OrderState>,
    ) -> Result<(), ReconcilerError> {
        use instant_acme::AuthorizationStatus;

        info!(%cert_id, "acme: setting challenge(s) to ready");

        let order_state =
            order_state.ok_or_else(|| ReconcilerError::OrderStore("missing order state".into()))?;

        // Re-register all challenge tokens so the CA can reach them even if the
        // process restarted between step_select_http01 and this step.
        for (token, key_auth) in &order_state.challenge_tokens {
            self.cert_manager
                .http01()
                .put(token.clone(), key_auth.clone());
        }

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

        // Notify the CA that every pending HTTP-01 challenge is ready.
        // For SAN orders with N domains all N pending authorizations must be
        // signalled; skipping any of them causes the CA to time-out on those
        // domains and eventually fail the entire order.
        let mut ready_count = 0usize;

        while let Some(res) = authzs.next().await {
            let mut authz = res.map_err(|e| ReconcilerError::Acme(e.to_string()))?;

            if authz.status != AuthorizationStatus::Pending {
                continue;
            }

            if let Some(mut challenge) = authz.challenge(instant_acme::ChallengeType::Http01) {
                challenge
                    .set_ready()
                    .await
                    .map_err(|e| ReconcilerError::Acme(e.to_string()))?;

                ready_count += 1;
            }
        }

        if ready_count == 0 {
            return Err(ReconcilerError::NoHttp01Challenge);
        }

        let mut new_state = order_state.clone();
        new_state.status = OrderStatus::Challenging;
        new_state.updated_at = std::time::SystemTime::now();

        let store = self.cert_manager.order_store();
        tokio::task::spawn_blocking(move || store.put(&new_state))
            .await
            .map_err(|e| ReconcilerError::OrderStore(format!("join error: {e}")))?
            .map_err(|e| ReconcilerError::OrderStore(format!("io error: {e}")))?;

        info!(%cert_id, ready_count, "acme: all pending challenges set to ready; entering Challenging state");
        Ok(())
    }

    async fn step_poll_ready(
        &self,
        cert_id: String,
        order_state: Option<OrderState>,
    ) -> Result<(), ReconcilerError> {
        use instant_acme::{OrderStatus as AcmeOrderStatus, RetryPolicy};

        debug!(%cert_id, "acme: polling order readiness");

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
            debug!(%cert_id, "acme: order not ready yet; will retry");
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

        info!(%cert_id, "acme: order ready; entering Finalizing state");
        Ok(())
    }

    async fn step_finalize_and_store(
        &self,
        cert_id: String,
        desired: DesiredCertificate,
        order_state: Option<OrderState>,
    ) -> Result<(), ReconcilerError> {
        use instant_acme::RetryPolicy;

        info!(%cert_id, "acme: finalizing order");

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

        let not_after = parse_not_after(&cert_chain_pem).map_err(|e| {
            ReconcilerError::Acme(format!(
                "cannot parse certificate to extract expiration: {e}"
            ))
        })?;
        info!(%cert_id, "acme: certificate not_after: {:?}", not_after);

        let meta = CertificateMeta {
            domains: desired.domains,
            not_after,
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

        info!(%cert_id, "acme: certificate stored successfully");

        for (token, _) in &order_state.challenge_tokens {
            self.cert_manager.http01().remove(token);
        }

        let order_store = self.cert_manager.order_store();
        let id = cert_id.clone();
        tokio::task::spawn_blocking(move || order_store.delete(&id))
            .await
            .map_err(|e| ReconcilerError::OrderStore(format!("join error: {e}")))?
            .map_err(|e| ReconcilerError::OrderStore(format!("io error: {e}")))?;

        debug!(%cert_id, "acme: order state cleaned up");

        // Rebuild and publish SNI map so new handshakes see the cert immediately.
        let cm = self.cert_manager.clone();
        let new_map = tokio::task::spawn_blocking(move || cm.build_sni_map())
            .await
            .map_err(|e| ReconcilerError::CertStore(format!("join error rebuilding sni map: {e}")))?
            .map_err(|e| ReconcilerError::CertStore(format!("rebuild sni map: {e}")))?;

        // Publish on the data plane boundary (lock-free for handshakes).
        self.cert_manager.publish_sni_map(new_map);

        info!(%cert_id, "acme: published updated SNI map");

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DesiredCertificate {
    pub domains: Vec<String>,
    pub challenge: AcmeChallengeConfig,
}

fn desired_certificates_from_config(
    config: &RuntimeConfig,
) -> BTreeMap<String, DesiredCertificate> {
    let mut out = BTreeMap::new();

    for l in &config.listeners {
        let Some(certificate_config) = &l.tls_termination else {
            continue;
        };

        if let TlsTerminationConfig::Acme { domains, challenge } = certificate_config {
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

fn compute_cert_id(domains: &[String], challenge: &AcmeChallengeConfig) -> String {
    let mut hasher = Sha256::new();

    for d in domains {
        hasher.update(d.as_bytes());
        hasher.update(b"\0");
    }

    hasher.update(format!("{challenge:?}").as_bytes());

    let digest = hasher.finalize();
    format!("{:x}", digest)[..32].to_string()
}

fn parse_not_after(cert_chain_pem: &str) -> anyhow::Result<std::time::SystemTime> {
    let (_, pem) = parse_x509_pem(cert_chain_pem.as_bytes())?;
    let (_, cert) = parse_x509_certificate(&pem.contents)?;
    let not_after = cert.validity().not_after.to_datetime();
    let secs = not_after.unix_timestamp(); // i64
    if secs < 0 {
        anyhow::bail!("certificate not_after predates Unix epoch: {secs}s");
    }
    Ok(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs as u64))
}
