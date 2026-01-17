//! Log Processing Pipeline
//!
//! This module handles reading and displaying log messages from Snakeway in different ways.
//!
//! Think of it like a filter for a stream of water - log messages flow in from the input,
//! and this code decides how to show them to you based on what you asked for.
//!
//! There are three ways to view logs:
//! - **Raw mode**: Shows the logs exactly as they come in, no changes
//! - **Pretty mode**: Makes the logs easier to read by formatting them nicely
//! - **Stats mode**: Instead of showing every single log, it counts things up and shows
//!   you a summary every second - like how many requests per second, how fast they were,
//!   and whether anything went wrong
//!
//! The code reads log messages one line at a time, figures out what kind of message it is
//! (either a web request or a general system message), and then either displays it nicely
//! or adds it to the running statistics counter.
//!
//! For stats mode, it keeps track of recent events in a sliding time window (like looking
//! at the last 10 seconds) and calculates things like request speed and response times.
//!
//!
//! The overall data processing architecture is:
//!
//! stdin
//! parse_event
//! LogEvent
//! StatsAggregator
//! StatsSnapshot
//! render_stats
//!

mod constants;
mod histogram;
mod parse;
mod render;
mod run;
mod stats_aggregation;
mod types;

pub use run::run_logs;
