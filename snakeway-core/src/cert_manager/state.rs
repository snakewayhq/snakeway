use crate::cert_manager::renewal_policy::RenewalPolicy;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertState {
    /// No certificate exists in the store and no ACME order is active.
    ///
    /// This is the starting state for a domain that requires TLS.
    /// Reconciliation should initiate a new ACME order.
    Absent,

    /// An ACME order has been created with the CA, but no challenges
    /// have yet been initialized.
    ///
    /// This represents the moment immediately after creating the order.
    Ordering,

    /// The ACME order exists and required challenges have been selected
    /// and prepared locally (e.g. HTTP-01 token written to challenge store),
    /// but validation has not yet begun.
    ///
    /// Transitional state before the CA validates the challenge.
    ChallengeInit,

    /// The ACME challenge is currently being validated by the CA.
    ///
    /// The system is waiting for the CA to mark the authorization as valid
    /// or invalid. No new order should be created while in this state.
    Challenging,

    /// The challenge(s) succeeded and the order is ready to be finalized.
    ///
    /// The CSR has been submitted and the system is waiting for the
    /// certificate to be issued by the CA.
    Finalizing,

    /// A certificate exists in the store and is currently valid
    /// (i.e. not expired and not within the renewal window).
    ///
    /// This is the steady-state for a healthy domain.
    Valid,

    /// A certificate exists and is approaching expiration based on
    /// renewal policy (e.g. < 30 days remaining).
    ///
    /// Reconciliation should initiate a new ACME order.
    /// Once an order is started, state transitions into Ordering.
    Renewing,

    /// The last ACME attempt failed (challenge failure, CA rejection,
    /// network error, etc.).
    ///
    /// The system should apply backoff before retrying.
    /// Must not continuously retry every tick.
    Failed,
}

/// v1 state computation based on presence and expiry.
/// Later fold in "pending orders" and failure counters from a separate order store.
pub fn compute_state(
    cert_id: &str,
    meta: Option<&crate::cert_manager::store::CertificateMeta>,
    renewal_policy: &RenewalPolicy,
) -> CertState {
    let Some(meta) = meta else {
        return CertState::Absent;
    };

    let renew_within = renewal_policy.renew_within;

    let now = SystemTime::now();
    match meta.not_after.duration_since(now) {
        Ok(time_left) => {
            if time_left <= renew_within {
                CertState::Renewing
            } else {
                CertState::Valid
            }
        }
        Err(_) => {
            // not_after is in the past
            CertState::Renewing
        }
    }
}
