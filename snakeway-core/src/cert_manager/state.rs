#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertState {
    Absent,
    Ordering,
    ChallengeInit,
    Challenging,
    Finalizing,
    Valid,
    Renewing,
    Failed,
}

pub fn compute_state(
    cert_id: &str,
    actual: &[(String, super::store::CertificateMeta)],
) -> CertState {
    todo!()
}
