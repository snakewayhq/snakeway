use crate::ctx::request::NormalizedRequest;

pub enum NormalizationOutcome {
    Accept(NormalizedRequest),
    Rewrite {
        request: NormalizedRequest,
        reason: RewriteReason,
    },
    Reject {
        reason: RejectReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    InvalidUtf8,
    PathTraversal,
    InvalidPercentEncoding,
    InvalidQueryEncoding,
    HeaderEncodingViolation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewriteReason {
    PathCollapse,
    DotSegmentRemoval,
    PercentDecodeUnreserved,
    QueryCanonicalization,
}
