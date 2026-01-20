use crate::ctx::request::NormalizedHeaders;
use crate::ctx::request::normalization::NormalizationOutcome;
use http::HeaderMap;

pub fn normalize_headers(raw: &HeaderMap) -> NormalizationOutcome<NormalizedHeaders> {
    todo!("normalize headers")
}
