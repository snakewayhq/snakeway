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

use crate::ctx::RequestId;
use crate::logging::LogMode;
use anyhow::Result;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::io::{self, BufRead, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

const WINDOW: Duration = Duration::from_secs(10);
const RENDER_TICK: Duration = Duration::from_secs(1);
const IN_FLIGHT_TTL: Duration = Duration::from_secs(60);
const LOOP_IDLE_SLEEP: Duration = Duration::from_millis(25);

pub fn run_logs(mode: LogMode) -> Result<()> {
    match mode {
        LogMode::Raw => run_raw(),
        LogMode::Pretty => run_pretty(),
        LogMode::Stats => run_stats(),
    }
}

fn run_raw() -> Result<()> {
    let stdin = io::stdin();
    let reader = stdin.lock();

    for line in reader.lines() {
        println!("{}", line?);
    }
    Ok(())
}

fn run_pretty() -> Result<()> {
    let stdin = io::stdin();
    let reader = stdin.lock();

    for line in reader.lines() {
        let line = line?;

        let Ok(json) = serde_json::from_str::<Value>(&line) else {
            // Preserve non-JSON lines as-is for troubleshooting.
            println!("{line}");
            continue;
        };

        if let Some(event) = parse_event(&json) {
            render_pretty(&event);
        }
    }
    Ok(())
}

fn run_stats() -> Result<()> {
    // Channel from reader thread -> stats loop.
    let (tx, rx) = mpsc::channel::<LogEvent>();

    // Reader thread: stdin -> parse -> send(LogEvent)
    let reader_handle = thread::spawn(move || {
        let stdin = io::stdin();
        let reader = stdin.lock();

        for line in reader.lines().flatten() {
            let Ok(json) = serde_json::from_str::<Value>(&line) else {
                // Ignore non-JSON in stats mode (keeps dashboard clean).
                continue;
            };

            if let Some(event) = parse_event(&json) {
                // If receiver is gone, stop early.
                if tx.send(event).is_err() {
                    break;
                }
            }
        }
        // tx is dropped here, which will disconnect rx.
    });

    // Optional UX polish: hide cursor while dashboard runs.
    print!("\x1b[?25l");
    let _ = io::stdout().flush();

    let mut agg = StatsAggregator::new(WINDOW);
    let mut last_render = Instant::now();

    // Stats render loop
    loop {
        let mut disconnected = false;

        // Drain events
        loop {
            match rx.try_recv() {
                Ok(ev) => agg.push(&ev),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }

        if last_render.elapsed() >= RENDER_TICK {
            let snap = agg.snapshot();
            redraw(&render_stats(&snap));
            last_render = Instant::now();
        }

        if disconnected {
            break;
        }

        thread::sleep(LOOP_IDLE_SLEEP);
    }

    // Restore cursor
    print!("\x1b[?25h");
    let _ = io::stdout().flush();

    // Join reader thread (best effort)
    let _ = reader_handle.join();

    Ok(())
}

//-----------------------------------------------------------------------------
// Event model
//-----------------------------------------------------------------------------

#[derive(Clone)]
enum LogEvent {
    Snakeway(SnakewayEvent),
    Generic(GenericEvent),
}

#[derive(Clone)]
struct SnakewayEvent {
    request_id: Option<String>,
    level: String,
    name: String,
    method: Option<String>,
    uri: Option<String>,
    status: Option<i64>, // status is a string in logs; we parse to i64
    ts: Option<SystemTime>,
    identity: Option<IdentitySummary>,
}

#[derive(Clone, Default)]
struct IdentitySummary {
    device: Option<String>,
    bot: Option<bool>,
}

#[derive(Clone)]
struct GenericEvent {
    level: String,
    message: String,
    target: Option<String>,
}

//-----------------------------------------------------------------------------
// Parsing
//-----------------------------------------------------------------------------

fn is_snakeway_event(event: &Value) -> bool {
    event.get("method").is_some() || event.get("uri").is_some() || event.get("status").is_some()
}

fn parse_event(event: &Value) -> Option<LogEvent> {
    let level = event
        .get("level")
        .and_then(Value::as_str)
        .unwrap_or("INFO")
        .to_string();

    if is_snakeway_event(event) {
        Some(LogEvent::Snakeway(SnakewayEvent {
            level,
            request_id: event
                .get("request_id")
                .and_then(Value::as_str)
                .map(str::to_string),
            ts: event
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(SystemTime::from),
            identity: event.get("identity").and_then(|v| {
                // identity is a JSON-encoded string
                let s = v.as_str()?;
                let Ok(parsed) = serde_json::from_str::<Value>(s) else {
                    return None;
                };

                let device = parsed
                    .get("device")
                    .and_then(Value::as_str)
                    .map(String::from);

                // In your logs bot is a string ("true"/"false")
                let bot = parsed
                    .get("bot")
                    .and_then(Value::as_str)
                    .and_then(|b| b.parse::<bool>().ok());

                if device.is_some() || bot.is_some() {
                    Some(IdentitySummary { device, bot })
                } else {
                    None
                }
            }),
            name: event
                .get("event")
                .and_then(Value::as_str)
                .unwrap_or("request")
                .to_string(),
            method: event
                .get("method")
                .and_then(Value::as_str)
                .map(str::to_string),
            uri: event.get("uri").and_then(Value::as_str).map(str::to_string),
            // status is a string in your logs (e.g. "200")
            status: event
                .get("status")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<i64>().ok()),
        }))
    } else {
        Some(LogEvent::Generic(GenericEvent {
            level,
            message: event
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("<no message>")
                .to_string(),
            target: event
                .get("target")
                .and_then(Value::as_str)
                .map(str::to_string),
        }))
    }
}

//-----------------------------------------------------------------------------
// Pretty rendering
//-----------------------------------------------------------------------------

fn render_pretty(event: &LogEvent) {
    match event {
        LogEvent::Snakeway(e) => {
            print!("[{}] {}", e.level, e.name);
            if let (Some(m), Some(u)) = (&e.method, &e.uri) {
                print!(" → {m} {u}");
            }
            if let Some(s) = e.status {
                print!(" ({s})");
            }
            println!();
        }
        LogEvent::Generic(e) => {
            if let Some(target) = &e.target {
                println!("[{}] {} ({})", e.level, e.message, target);
            } else {
                println!("[{}] {}", e.level, e.message);
            }
        }
    }
}

//-----------------------------------------------------------------------------
// Stats rendering
//-----------------------------------------------------------------------------

fn render_stats(snapshot: &StatsSnapshot) -> String {
    let mut out = String::new();

    let (_ok, _client, server) = snapshot.status;

    out.push_str(&format!(
        "Snakeway Stats ({}s window)\n\
         ==========================\n\
         RPS: {:.1} | events: {} | 5xx: {}\n\n",
        snapshot.window_seconds, snapshot.rps, snapshot.window_events, server
    ));

    let total_latency: u64 = snapshot.latency.iter().map(|(_, c)| *c).sum();
    if total_latency > 0 {
        out.push_str("Latency (window):\n");
        for (label, count) in &snapshot.latency {
            let pct = (*count as f64 / total_latency as f64) * 100.0;
            let bars = ((pct / 5.0).floor() as usize).max(1);
            out.push_str(&format!(
                "  {:<8} {:<20} {:>5.1}%\n",
                label,
                "█".repeat(bars),
                pct
            ));
        }
        out.push('\n');
    } else {
        out.push_str("Latency (window): <no samples>\n\n");
    }

    out.push_str(&format!(
        "Latency p95 ≈ {}ms | p99 ≈ {}ms\n\n",
        snapshot.p95_ms, snapshot.p99_ms
    ));

    // Identity semantics: these are counts of events with bot info present.
    out.push_str(&format!(
        "Identity: human={} bot={} unknown={}\n",
        snapshot.human_count, snapshot.bot_count, snapshot.unknown_identity_count
    ));

    if !snapshot.device_counts.is_empty() {
        // stable ordering: by device name
        let mut devices: Vec<_> = snapshot.device_counts.iter().collect();
        devices.sort_by_key(|(device, _)| *device);

        out.push_str("Devices: ");
        for (d, c) in devices {
            out.push_str(&format!("{d}={c} "));
        }
        out.push('\n');
    }

    let (ok, client, server) = snapshot.status;
    out.push_str(&format!(
        "\nStatus: 2xx={} 4xx={} 5xx={}\n",
        ok, client, server
    ));

    out
}

fn redraw(output: &str) {
    print!("\x1b[2J\x1b[H");
    println!("{output}");
    let _ = io::stdout().flush();
}

//-----------------------------------------------------------------------------
// Stats aggregation
//-----------------------------------------------------------------------------

struct WindowEvent {
    inserted_at: Instant,    // for eviction
    latency_ms: Option<u64>, // computed from timestamps when available
    status: Option<i64>,
    identity: IdentitySummary,
}

struct StatsAggregator {
    window: Duration,
    events: VecDeque<WindowEvent>,
    in_flight: HashMap<RequestId, InFlight>,
}

struct InFlight {
    start_instant: Instant,           // for TTL eviction
    start_system: Option<SystemTime>, // for latency math
    status: Option<i64>,
    identity: IdentitySummary,
}

impl StatsAggregator {
    fn new(window: Duration) -> Self {
        Self {
            window,
            events: VecDeque::new(),
            in_flight: HashMap::new(),
        }
    }

    fn push(&mut self, event: &LogEvent) {
        let LogEvent::Snakeway(e) = event else {
            return;
        };
        let Some(request_id) = &e.request_id else {
            return;
        };

        let request_id = RequestId(request_id.clone());

        match e.name.as_str() {
            "request" => {
                self.in_flight.entry(request_id).or_insert(InFlight {
                    start_instant: Instant::now(),
                    start_system: e.ts,
                    status: None,
                    identity: e.identity.clone().unwrap_or_default(),
                });
            }
            "after_proxy" => {
                if let Some(f) = self.in_flight.get_mut(&request_id) {
                    f.status = e.status;
                }
            }
            "response" => {
                if let Some(f) = self.in_flight.remove(&request_id) {
                    let latency_ms = match (e.ts, f.start_system) {
                        (Some(end), Some(start)) => {
                            end.duration_since(start).ok().map(|d| d.as_millis() as u64)
                        }
                        _ => None,
                    };

                    self.events.push_back(WindowEvent {
                        inserted_at: Instant::now(),
                        latency_ms,
                        status: e.status.or(f.status),
                        identity: f.identity,
                    });
                }
            }
            _ => {}
        }
    }

    fn evict_window(&mut self, now: Instant) {
        while let Some(ev) = self.events.front() {
            if now.duration_since(ev.inserted_at) > self.window {
                self.events.pop_front();
            } else {
                break;
            }
        }
    }

    fn evict_in_flight(&mut self, now: Instant) {
        self.in_flight
            .retain(|_, f| now.duration_since(f.start_instant) <= IN_FLIGHT_TTL);
    }

    fn snapshot(&mut self) -> StatsSnapshot {
        let now = Instant::now();
        self.evict_window(now);
        self.evict_in_flight(now);

        let mut latency = Histogram::new(LATENCY_BUCKETS_MS);
        let mut status_2xx = 0;
        let mut status_4xx = 0;
        let mut status_5xx = 0;

        let mut device_counts: HashMap<String, u64> = HashMap::new();
        let mut bot_count = 0;
        let mut human_count = 0;
        let mut unknown_identity_count = 0;

        for ev in &self.events {
            if let Some(ms) = ev.latency_ms {
                latency.record(ms);
            }

            if let Some(status) = ev.status {
                match status {
                    200..=299 => status_2xx += 1,
                    400..=499 => status_4xx += 1,
                    500..=599 => status_5xx += 1,
                    _ => {}
                }
            }

            match ev.identity.bot {
                Some(true) => bot_count += 1,
                Some(false) => human_count += 1,
                None => unknown_identity_count += 1,
            }

            if let Some(device) = &ev.identity.device {
                *device_counts.entry(device.clone()).or_insert(0) += 1;
            }
        }

        let buckets = latency.numeric_buckets();
        let total_latency: u64 = buckets.iter().map(|(_, c)| *c).sum();

        let p95_ms = percentile_from_histogram(&buckets, total_latency, 0.95);
        let p99_ms = percentile_from_histogram(&buckets, total_latency, 0.99);

        // RPS: use observed span, but avoid lying for sub-second spans by clamping to 0.1s.
        let span = self
            .events
            .back()
            .zip(self.events.front())
            .map(|(b, f)| b.inserted_at.duration_since(f.inserted_at))
            .unwrap_or(self.window);

        let denom = span.as_secs_f64().clamp(0.1, self.window.as_secs_f64());
        let rps = self.events.len() as f64 / denom;

        StatsSnapshot {
            window_seconds: self.window.as_secs().max(1),
            rps,
            window_events: self.events.len() as u64,
            latency: latency.snapshot(),
            status: (status_2xx, status_4xx, status_5xx),
            p95_ms,
            p99_ms,
            device_counts,
            bot_count,
            human_count,
            unknown_identity_count,
        }
    }
}

struct StatsSnapshot {
    window_seconds: u64,

    rps: f64,
    window_events: u64,
    latency: Vec<(String, u64)>,
    status: (u64, u64, u64), // 2xx, 4xx, 5xx

    p95_ms: u64,
    p99_ms: u64,

    device_counts: HashMap<String, u64>,
    bot_count: u64,
    human_count: u64,
    unknown_identity_count: u64,
}

//-----------------------------------------------------------------------------
// Histograms
//-----------------------------------------------------------------------------

const LATENCY_BUCKETS_MS: &[u64] = &[1, 5, 10, 25, 50, 100, 250, 500, 1000];

#[derive(Clone)]
struct Histogram {
    buckets: &'static [u64],
    counts: Vec<u64>,
}

impl Histogram {
    fn new(buckets: &'static [u64]) -> Self {
        Self {
            buckets,
            counts: vec![0; buckets.len() + 1], // +∞ bucket
        }
    }

    fn record(&mut self, value: u64) {
        for (i, b) in self.buckets.iter().enumerate() {
            if value <= *b {
                self.counts[i] += 1;
                return;
            }
        }
        *self.counts.last_mut().unwrap() += 1;
    }

    fn snapshot(&self) -> Vec<(String, u64)> {
        let mut out = Vec::new();

        for (i, c) in self.counts.iter().enumerate() {
            let label = if i == 0 {
                format!("0–{}ms", self.buckets[0])
            } else if i < self.buckets.len() {
                format!("{}–{}ms", self.buckets[i - 1] + 1, self.buckets[i])
            } else {
                format!(">{}ms", self.buckets.last().unwrap())
            };

            out.push((label, *c));
        }

        out
    }

    fn numeric_buckets(&self) -> Vec<(u64, u64)> {
        let mut out = Vec::new();

        for (i, count) in self.counts.iter().enumerate() {
            let upper = if i < self.buckets.len() {
                self.buckets[i]
            } else {
                u64::MAX // overflow bucket
            };
            out.push((upper, *count));
        }

        out
    }
}

fn percentile_from_histogram(buckets: &[(u64, u64)], total: u64, pct: f64) -> u64 {
    if total == 0 {
        return 0;
    }

    let target = (total as f64 * pct).ceil() as u64;
    let mut running = 0;

    for (upper, count) in buckets {
        running += *count;
        if running >= target {
            if *upper == u64::MAX {
                // "greater than last real bucket"
                return buckets
                    .iter()
                    .rev()
                    .find(|(u, _)| *u != u64::MAX)
                    .map(|(u, _)| u.saturating_add(1))
                    .unwrap_or(0);
            }
            return *upper;
        }
    }

    0
}
