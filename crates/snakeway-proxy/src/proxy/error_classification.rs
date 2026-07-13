use snakeway_engine::traffic::TransportFailure;

/// Classifies Pingora upstream errors into Snakeway transport failures.
/// Non-upstream errors are intentionally ignored to avoid penalizing healthy upstreams.
pub(crate) fn classify_pingora_error(err: &pingora::Error) -> Option<TransportFailure> {
    use pingora::{ErrorSource, ErrorType::*};

    // Only penalize upstream-originated errors.
    if err.esource() != &ErrorSource::Upstream {
        return None;
    }

    let failure = match err.etype() {
        // Connect phase.
        ConnectTimedout | ConnectRefused | ConnectNoRoute | ConnectProxyFailure | ConnectError => {
            TransportFailure::Connect
        }

        // TLS handshake.
        TLSHandshakeFailure | TLSHandshakeTimedout | TLSWantX509Lookup | InvalidCert
        | HandshakeError => TransportFailure::Tls,

        // Protocol violations.
        InvalidHTTPHeader | H1Error | H2Error | InvalidH2 | H2Downgrade => {
            TransportFailure::Protocol
        }

        // Established connection I/O.
        ReadTimedout | WriteTimedout => TransportFailure::Timeout,

        ReadError | WriteError | ConnectionClosed => TransportFailure::Reset,

        // Non-upstream-related errors.
        BindError
        | AcceptError
        | SocketError
        | FileOpenError
        | FileCreateError
        | FileReadError
        | FileWriteError
        | InternalError
        | UnknownError
        | Custom(_)
        | CustomCode(_, _)
        | HTTPStatus(_) => {
            return None;
        }
    };

    Some(failure)
}
