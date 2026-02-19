#[derive(Debug)]
pub enum AcmeError {
    RateLimited,
    InvalidOrder,
    InvalidChallenge,
    AuthorizationFailed,
    Network(std::io::Error),
    Protocol(String),
    Unexpected(String),
}
