use openssl::pkey::{PKey, Private};
use openssl::x509::X509;

pub struct ParsedCert {
    /// Leaf certificate (the server cert).
    pub leaf: X509,

    /// Intermediate chain (does NOT include leaf).
    pub chain: Vec<X509>,

    /// Private key.
    pub key: PKey<Private>,
}
