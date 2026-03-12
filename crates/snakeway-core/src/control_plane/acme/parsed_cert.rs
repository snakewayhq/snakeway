use openssl::pkey::{PKey, Private};
use openssl::x509::X509;

pub(crate) struct ParsedCert {
    /// Leaf certificate (the server cert).
    pub(crate) leaf: X509,

    /// Intermediate chain (does NOT include leaf).
    pub(crate) chain: Vec<X509>,

    /// Private key.
    pub(crate) key: PKey<Private>,
}
