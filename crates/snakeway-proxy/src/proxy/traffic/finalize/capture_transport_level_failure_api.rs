use crate::proxy::TrafficProxy;
use pingora::Error;
use snakeway_engine::ctx::RequestCtx;
use snakeway_engine::traffic::{TransportFailure, UpstreamOutcome};

impl TrafficProxy {
    /// Classify a pingora/transport error and set it as the upstream outcome.
    pub(crate) fn capture_transport_level_failure(&self, ctx: &mut RequestCtx, e: Option<&Error>) {
        if let Some(err) = e
            && let Some(failure) = classify_pingora_error(err)
        {
            ctx.upstream_outcome = Some(UpstreamOutcome::Transport(failure));
        }
    }
}

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
