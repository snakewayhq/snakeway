use super::stats_aggregation::StatsSnapshot;
use crate::cli::logs::types::LogEvent;
use std::io;
use std::io::Write;

pub fn render_stats(snapshot: &StatsSnapshot) -> String {
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

pub fn redraw(output: &str) {
    print!("\x1b[2J\x1b[H");
    println!("{output}");
    let _ = io::stdout().flush();
}

pub fn render_pretty(event: &LogEvent) {
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
