use crate::ctx::request::normalization::{NormalizationOutcome, normalize_path};
use crate::ctx::request::{CanonicalQuery, NormalizedHeaders, NormalizedRequest};
use pingora::http::RequestHeader;

/// Normalize a Pingora request into a deterministic, safe representation.
pub fn normalize_request(req: &RequestHeader) -> NormalizationOutcome<NormalizedRequest> {
    // Normalize the path...
    let path_outcome = normalize_path(req.uri.path());

    // todo Build the canonical query
    let query = CanonicalQuery::new(req.uri.query().unwrap_or(""));

    // todo normalize the headers
    let headers = NormalizedHeaders;

    match path_outcome {
        NormalizationOutcome::Accept(path) => NormalizationOutcome::Accept(NormalizedRequest::new(
            req.method.clone(),
            path,
            query,
            headers,
        )),

        NormalizationOutcome::Rewrite {
            value: path,
            reason,
        } => NormalizationOutcome::Rewrite {
            value: NormalizedRequest::new(req.method.clone(), path, query, headers),
            reason,
        },

        NormalizationOutcome::Reject { reason } => NormalizationOutcome::Reject { reason },
    }
}
