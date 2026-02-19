#[derive(Debug)]
pub enum CertManagerError {
    StoreError(String),
    AcmeError(String),
    ChallengeError(String),
}
