//! The read side of the replay engine: reading responses to their boundary.

use super::plan::{Exchange, Outcome, ReadPolicy, WritePlan};
use super::response::{BodyKind, ResponseHead, parse_chunked, parse_response_head};
use std::io::Read;
use std::net::TcpStream;
use std::time::{Duration, Instant};

/// The default grace window the engine waits after a complete response
/// boundary for extra bytes. Tests hunting a late second response pass a
/// bound of their own through `replay_with_grace`.
pub const DEFAULT_TRAILING_GRACE: Duration = Duration::from_millis(250);

/// What one read attempt produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// Bytes arrived.
    Progress,
    /// The peer closed the connection.
    Eof,
    /// The read timeout passed.
    Timeout,
}

pub(crate) fn read_exchange(
    stream: &mut TcpStream,
    plan: &WritePlan,
    policy: ReadPolicy,
    timeout: Duration,
    trailing_grace: Duration,
) -> Exchange {
    let deadline = Instant::now() + timeout;
    let mut raw = Vec::new();
    let head_request = is_head(plan);
    match policy {
        ReadPolicy::UntilClose => read_until_close(stream, &mut raw, deadline),
        ReadPolicy::OneResponse => {
            read_responses(stream, &mut raw, deadline, 1, head_request, trailing_grace)
        }
        ReadPolicy::Responses(count) => read_responses(
            stream,
            &mut raw,
            deadline,
            count,
            head_request,
            trailing_grace,
        ),
    }
}

/// A HEAD request gets a response with no body, so the boundary is the end of
/// the response head. The method is read from the request bytes the plan
/// carries, across chunk boundaries, which keeps raw mode honest about what
/// was sent.
fn is_head(plan: &WritePlan) -> bool {
    let mut head = Vec::new();
    for chunk in &plan.chunks {
        let room = 5usize.saturating_sub(head.len());
        head.extend_from_slice(&chunk[..room.min(chunk.len())]);
        if head.len() >= 5 {
            break;
        }
    }
    head.starts_with(b"HEAD ")
}

fn read_until_close(stream: &mut TcpStream, raw: &mut Vec<u8>, deadline: Instant) -> Exchange {
    let outcome = loop {
        match read_once(stream, raw, deadline) {
            Step::Progress => continue,
            Step::Eof => break Outcome::Closed,
            Step::Timeout => break Outcome::TimedOut,
        }
    };
    Exchange {
        raw: std::mem::take(raw),
        trailing: Vec::new(),
        outcome,
        closed_after_response: false,
    }
}

fn read_responses(
    stream: &mut TcpStream,
    raw: &mut Vec<u8>,
    deadline: Instant,
    count: usize,
    is_head: bool,
    trailing_grace: Duration,
) -> Exchange {
    let mut cursor = 0usize;
    let mut completed = 0usize;
    let mut outcome = Outcome::Closed;

    while completed < count {
        let Some(mut head) = wait_for_head(stream, raw, cursor, deadline, &mut outcome) else {
            break;
        };
        while head.is_interim() {
            cursor = head.head_len;
            match wait_for_head(stream, raw, cursor, deadline, &mut outcome) {
                Some(next) => head = next,
                None => break,
            }
        }
        if head.is_interim() {
            break;
        }
        cursor = match read_body_to_boundary(stream, raw, &head, is_head, deadline, &mut outcome) {
            Some(end) => end,
            None => break,
        };
        completed += 1;
        outcome = Outcome::Status(head.status);
    }

    let mut trailing = Vec::new();
    let mut closed_after_response = false;
    if completed == count && matches!(outcome, Outcome::Status(_)) {
        closed_after_response =
            drain_after_boundary(stream, &mut trailing, deadline, trailing_grace);
    }

    Exchange {
        raw: std::mem::take(raw),
        trailing,
        outcome,
        closed_after_response,
    }
}

/// Reads until a complete response head is available at `cursor`.
/// On EOF or timeout the outcome is recorded and None is returned.
fn wait_for_head(
    stream: &mut TcpStream,
    raw: &mut Vec<u8>,
    cursor: usize,
    deadline: Instant,
    outcome: &mut Outcome,
) -> Option<ResponseHead> {
    loop {
        if let Some(head) = parse_response_head(raw, cursor) {
            return Some(head);
        }
        match read_once(stream, raw, deadline) {
            Step::Progress => continue,
            Step::Eof => {
                *outcome = Outcome::Closed;
                return None;
            }
            Step::Timeout => {
                *outcome = Outcome::TimedOut;
                return None;
            }
        }
    }
}

/// Reads the response body of `head` to its boundary and returns the absolute
/// offset just past it. Responses without a framing rule read until close.
fn read_body_to_boundary(
    stream: &mut TcpStream,
    raw: &mut Vec<u8>,
    head: &ResponseHead,
    is_head: bool,
    deadline: Instant,
    outcome: &mut Outcome,
) -> Option<usize> {
    let kind = head.body_kind(is_head);
    match kind {
        BodyKind::None => Some(head.head_len),
        BodyKind::Sized(len) => {
            let end = head.head_len + len as usize;
            loop {
                if raw.len() >= end {
                    return Some(end);
                }
                match read_once(stream, raw, deadline) {
                    Step::Progress => continue,
                    Step::Eof => {
                        *outcome = Outcome::Closed;
                        return None;
                    }
                    Step::Timeout => {
                        *outcome = Outcome::TimedOut;
                        return None;
                    }
                }
            }
        }
        BodyKind::Chunked => loop {
            match parse_chunked(raw, head.head_len) {
                Some(end) => return Some(end),
                None => match read_once(stream, raw, deadline) {
                    Step::Progress => continue,
                    Step::Eof => {
                        *outcome = Outcome::Closed;
                        return None;
                    }
                    Step::Timeout => {
                        *outcome = Outcome::TimedOut;
                        return None;
                    }
                },
            }
        },
        BodyKind::UntilClose => loop {
            match read_once(stream, raw, deadline) {
                Step::Progress => continue,
                Step::Eof => {
                    *outcome = Outcome::Closed;
                    return Some(raw.len());
                }
                Step::Timeout => {
                    *outcome = Outcome::TimedOut;
                    return None;
                }
            }
        },
    }
}

/// One read attempt. The buffer keeps whatever arrived.
fn read_once(stream: &mut TcpStream, raw: &mut Vec<u8>, deadline: Instant) -> Step {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Step::Timeout;
    }
    let _ = stream.set_read_timeout(Some(remaining));
    let mut tmp = [0u8; 4096];
    match stream.read(&mut tmp) {
        Ok(0) => Step::Eof,
        Ok(n) => {
            raw.extend_from_slice(&tmp[..n]);
            Step::Progress
        }
        Err(e)
            if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
        {
            Step::Timeout
        }
        Err(_) => Step::Eof,
    }
}

/// After a complete response, reads whatever arrives within the grace window
/// and reports it as trailing bytes. EOF inside the window ends the drain
/// early and marks the response as followed by a close, so a test can
/// distinguish "closed after the response" from "kept alive". Returns that
/// close signal.
fn drain_after_boundary(
    stream: &mut TcpStream,
    trailing: &mut Vec<u8>,
    deadline: Instant,
    grace: Duration,
) -> bool {
    let grace_end = deadline.min(Instant::now() + grace);
    let _ = stream.set_read_timeout(Some(grace_end.saturating_duration_since(Instant::now())));
    let mut tmp = [0u8; 4096];
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => return true,
            Ok(n) => trailing.extend_from_slice(&tmp[..n]),
            Err(_) => return false,
        }
        if Instant::now() >= grace_end {
            return false;
        }
    }
}
