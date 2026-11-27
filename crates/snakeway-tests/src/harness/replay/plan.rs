//! The exchange data model: outcomes, write plans, and read policies.

use std::time::Duration;

/// How a complete exchange ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// A final response head was parsed and its body read to the boundary.
    Status(u16),
    /// The peer closed the connection before a complete response arrived.
    Closed,
    /// The read timeout passed before a complete response arrived. A timeout
    /// fails every rejection test, so tests never read a hang as success.
    TimedOut,
}

/// What one exchange produced.
#[derive(Debug, Clone)]
pub struct Exchange {
    /// Every byte read from the connection, in arrival order.
    pub raw: Vec<u8>,
    /// Bytes received after the boundary of the last response read. A test
    /// asserting that no second response appears reads this field.
    pub trailing: Vec<u8>,
    /// How the exchange ended.
    pub outcome: Outcome,
    /// True when the peer closed the connection after the response
    /// boundary, set from the trailing drain. False when the drain timed out
    /// with the connection still open or no drain ran.
    pub closed_after_response: bool,
}

impl Exchange {
    /// The final response status, or None on a close or a timeout.
    pub fn status(&self) -> Option<u16> {
        match self.outcome {
            Outcome::Status(code) => Some(code),
            _ => None,
        }
    }
}

/// How the engine writes the request bytes.
#[derive(Debug, Clone)]
pub struct WritePlan {
    pub(super) chunks: Vec<Vec<u8>>,
    pub(super) delay: Duration,
    pub(super) half_close: bool,
}

impl WritePlan {
    /// One write with every byte at once.
    pub fn single(bytes: Vec<u8>) -> Self {
        Self {
            chunks: vec![bytes],
            delay: Duration::ZERO,
            half_close: false,
        }
    }

    /// One write per slice, waiting `delay` between writes. The first write
    /// happens immediately.
    pub fn staged(chunks: Vec<Vec<u8>>, delay: Duration) -> Self {
        Self {
            chunks,
            delay,
            half_close: false,
        }
    }

    /// Shuts the write side after the last write, so the request ends in a
    /// half closed state.
    pub fn with_half_close(mut self) -> Self {
        self.half_close = true;
        self
    }
}

/// How much of the response stream the engine reads.
#[derive(Debug, Clone, Copy)]
pub enum ReadPolicy {
    /// Read exactly one response and stop at its boundary.
    OneResponse,
    /// Read N responses, stopping at each boundary.
    Responses(usize),
    /// Read until the peer closes or the timeout passes.
    UntilClose,
}
