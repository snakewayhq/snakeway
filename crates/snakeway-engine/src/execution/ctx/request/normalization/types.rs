#[derive(Debug)]
pub enum NormalizationOutcome<T> {
    Accept(T),
    Rewrite {
        value: T,
        // Semantically important, even if never read.
        #[allow(dead_code)]
        reason: RewriteReason,
    },
    Reject {
        // Semantically important, even if never read.
        #[allow(dead_code)]
        reason: RejectReason,
    },
}

impl<T> NormalizationOutcome<T> {
    #[inline]
    pub(crate) fn reject_for_header_encoding_violation() -> Self {
        Self::Reject {
            reason: RejectReason::HeaderEncodingViolation,
        }
    }

    #[inline]
    pub(crate) fn reject_for_smuggling_attempt() -> Self {
        Self::Reject {
            reason: RejectReason::RequestSmugglingAttempt,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    InvalidUtf8,
    PathTraversal,
    InvalidPercentEncoding,
    InvalidQueryEncoding,
    HeaderEncodingViolation,
    HopByHopHeader,
    RequestSmugglingAttempt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewriteReason {
    PathCanonicalization,
    QueryCanonicalization,
    HeaderCanonicalization,
    PercentDecodeUnreserved,
}

pub enum ProtocolNormalizationMode {
    Http1,
    Http2,
}
