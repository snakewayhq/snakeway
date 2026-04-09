use crate::control_plane::acme::order_store::{OrderState, OrderStatus};
use crate::control_plane::acme::renewal_policy::RenewalPolicy;
use serde::Serialize;
use std::fmt;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) enum CertState {
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

impl fmt::Display for CertState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CertState::Absent => write!(f, "Absent"),
            CertState::Ordering => write!(f, "Ordering"),
            CertState::ChallengeInit => write!(f, "ChallengeInit"),
            CertState::Challenging => write!(f, "Challenging"),
            CertState::Finalizing => write!(f, "Finalizing"),
            CertState::Valid => write!(f, "Valid"),
            CertState::Renewing => write!(f, "Renewing"),
            CertState::Failed => write!(f, "Failed"),
        }
    }
}

pub(crate) fn compute_state(
    meta: Option<&crate::control_plane::acme::cert_store::CertificateMeta>,
    order_state: Option<&OrderState>,
    renewal_policy: &RenewalPolicy,
) -> CertState {
    // If an ACME order exists, it overrides everything.
    if let Some(order) = order_state {
        return match order.status {
            OrderStatus::Ordering => CertState::Ordering,
            OrderStatus::ChallengeInit => CertState::ChallengeInit,
            OrderStatus::Challenging => CertState::Challenging,
            OrderStatus::Finalizing => CertState::Finalizing,
            OrderStatus::Failed => {
                // Apply simple exponential backoff
                let backoff_base_multiplier = 10;
                let backoff_secs = (1u64 << order.failure_count.min(16)) * backoff_base_multiplier;
                let retry_after = order
                    .updated_at
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .ok()
                    .and_then(|_| order.updated_at.elapsed().ok())
                    .map(|elapsed| elapsed.as_secs() >= backoff_secs)
                    .unwrap_or(false);

                if retry_after {
                    CertState::Absent
                } else {
                    CertState::Failed
                }
            }
        };
    }

    // No order in flight, derive from certificate presence and expiry.
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
            // expired
            CertState::Renewing
        }
    }
}
