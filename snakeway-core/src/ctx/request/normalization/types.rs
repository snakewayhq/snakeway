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

/// Small helper to reduce noisy call site boilerplate.
impl<T> NormalizationOutcome<T> {
    #[inline]
    pub fn reject_for_header_encoding_violation() -> Self {
        Self::Reject {
            reason: RejectReason::HeaderEncodingViolation,
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
