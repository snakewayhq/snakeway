use miette::Diagnostic;
use thiserror::Error;

#[derive(Clone, Debug, Error, Diagnostic)]
pub enum ConfigWarning {
    #[error("trusted_proxies contains a public IP range: {network}")]
    #[diagnostic(
        severity = "warning",
        help = "Public IP ranges should only be trusted if they belong to known infrastructure (e.g. CDN or load balancer):  {network}"
    )]
    PublicTrustedProxy { network: String },
}
