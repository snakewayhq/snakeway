use crate::traffic::TransportFailure;

pub fn classify_pingora_error(err: &pingora::Error) -> TransportFailure {
    use pingora::{ErrorSource, ErrorType::*};

    // Return unknown for downstream and internal errors (without penalizing upstream(s)).
    if err.esource() != &ErrorSource::Upstream {
        return TransportFailure::Unknown;
    }

    // Classify specific upstream errors from Pingora.
    match err.etype() {
        // Connect phase.
        ConnectTimedout | ConnectRefused | ConnectNoRoute | ConnectProxyFailure | ConnectError => {
            TransportFailure::Connect
        }

        // TLS / handshake.
        TLSHandshakeFailure | TLSHandshakeTimedout | TLSWantX509Lookup | InvalidCert
        | HandshakeError => TransportFailure::Tls,

        // Protocol.
        InvalidHTTPHeader | H1Error | H2Error | InvalidH2 | H2Downgrade => {
            TransportFailure::Protocol
        }

        // Established connection IO.
        ReadTimedout | WriteTimedout => TransportFailure::Timeout,

        ReadError | WriteError | ConnectionClosed => TransportFailure::Reset,

        // Everything else.
        _ => TransportFailure::Unknown,
    }
}
