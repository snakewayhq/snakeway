use super::errors::ReconcilerError;
use super::reconciler::Reconciler;
use crate::OrderStatus;
use crate::cert_store::StoredCertificate;
use crate::order_store::OrderState;
use tracing::{debug, error, info};

impl Reconciler {
    /// Publish one HTTP-01 challenge token so the CA's validation request can
    /// reach it.
    pub(super) fn register_challenge_token(&self, token: &str, key_auth: &str) {
        self.cert_manager
            .http01()
            .put(token.to_string(), key_auth.to_string());
    }
    /// Transition the persisted order to `Failed`, releasing its challenge
    /// tokens first so the abandoned order cannot leak them into the registry.
    /// The backoff logic in `compute_state` eventually returns `Absent`, which
    /// creates a fresh order.
    pub(super) async fn mark_order_failed(
        &self,
        order_state: Option<OrderState>,
        error: &ReconcilerError,
    ) {
        let Some(mut failed_state) = order_state else {
            return;
        };

        for (token, _) in &failed_state.challenge_tokens {
            self.cert_manager.http01().remove(token);
        }
        failed_state.challenge_tokens = Vec::new();
        failed_state.status = OrderStatus::Failed;
        failed_state.failure_count += 1;
        failed_state.last_error = Some(error.to_string());
        failed_state.updated_at = std::time::SystemTime::now();

        let store = self.cert_manager.order_store();
        if let Err(persist_err) = tokio::task::spawn_blocking(move || store.put(&failed_state))
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r.map_err(|e| e.to_string()))
        {
            error!(
                error = %persist_err,
                "cert_manager: failed to persist Failed state"
            );
        }
    }
    /// Persist the issued certificate chain and key to the cert store.
    pub(super) async fn store_certificate(
        &self,
        cert_id: String,
        stored: StoredCertificate,
    ) -> Result<(), ReconcilerError> {
        let cert_store = self.cert_manager.cert_store();
        let id = cert_id.clone();
        tokio::task::spawn_blocking(move || cert_store.put(id, stored))
            .await
            .map_err(|e| ReconcilerError::CertStore(format!("join error: {e}")))?
            .map_err(|e| ReconcilerError::CertStore(format!("io error: {e}")))?;

        info!(%cert_id, "acme: certificate stored successfully");
        Ok(())
    }
    /// Drop the completed order's persisted state.
    pub(super) async fn delete_order_state(&self, cert_id: &str) -> Result<(), ReconcilerError> {
        let order_store = self.cert_manager.order_store();
        let id = cert_id.to_string();
        tokio::task::spawn_blocking(move || order_store.delete(&id))
            .await
            .map_err(|e| ReconcilerError::OrderStore(format!("join error: {e}")))?
            .map_err(|e| ReconcilerError::OrderStore(format!("io error: {e}")))?;

        debug!(%cert_id, "acme: order state cleaned up");
        Ok(())
    }
}
