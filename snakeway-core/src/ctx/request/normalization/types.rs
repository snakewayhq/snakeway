#[derive(Debug)]
pub enum NormalizationOutcome<T> {
    Accept(T),
    Rewrite { value: T, reason: RewriteReason },
    Reject { reason: RejectReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    InvalidUtf8,
    PathTraversal,
    InvalidPercentEncoding,
    InvalidQueryEncoding,
    HeaderEncodingViolation,
    HopByHopHeader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewriteReason {
    PathCanonicalization,
    QueryCanonicalization,
    HeaderCanonicalization,
    PercentDecodeUnreserved,
}
