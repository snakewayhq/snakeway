//! Reads Snakeway log lines from stdin and presents them in one of three modes.
//!
//! Raw mode passes each line through untouched. Pretty mode parses every line into a
//! `LogEvent`, classifies it as a request or a general message, and reformats it for
//! reading. Stats mode feeds the same events into an aggregator that keeps a 10 second
//! sliding window and prints request rate, latency percentiles, status counts, and the
//! identity breakdown once per second.

mod constants;
mod histogram;
mod parse;
mod render;
mod run;
mod stats_aggregation;
mod types;

pub(crate) use run::run_logs;
