use pingora::listeners::TlsAcceptCallbacks;
use pingora_openssl::ssl::SslRef;
use std::sync::Arc;

use crate::cert_manager::CertStore;

pub fn build_tls_callbacks(store: Arc<dyn CertStore>) -> TlsAcceptCallbacks {
    TlsAcceptCallbacks::new(move |ssl: &mut SslRef| {
        // 1. Extract SNI
        let sni = ssl.servername(openssl::ssl::NameType::HOST_NAME);

        let hostname = match sni {
            Some(name) => name,
            None => {
                // Decide policy: return error
                return Err(openssl::error::ErrorStack::get());
            }
        };

        // 2. Lookup certificate
        let cert = match store.get(hostname) {
            Some(cert) => cert,
            None => {
                return Err(openssl::error::ErrorStack::get());
            }
        };

        // 3. Parse PEM into OpenSSL objects
        let x509 = openssl::x509::X509::from_pem(&cert.cert_chain_pem)
            .map_err(|_| openssl::error::ErrorStack::get())?;

        let pkey = openssl::pkey::PKey::private_key_from_pem(&cert.private_key_pem)
            .map_err(|_| openssl::error::ErrorStack::get())?;

        // 4. Install into SSL session
        ssl.set_certificate(&x509)?;
        ssl.set_private_key(&pkey)?;

        Ok(())
    })
}
