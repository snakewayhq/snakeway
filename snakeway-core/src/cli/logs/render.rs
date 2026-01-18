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

    let (ok, client, server) = snapshot.status;
    out.push_str(&format!(
        "\nStatus: 2xx={} 4xx={} 5xx={}\n",
        ok, client, server
    ));
    out.push_str("\n --------------------- \n");
    // Identity semantics: these are counts of events with bot info present.
    out.push_str(&format!(
        "Identity: human={} bot={} unknown={}\n",
        snapshot.human_count, snapshot.bot_count, snapshot.unknown_identity_count
    ));

    // Identity semantics: these are counts of events with device info present.
    if !snapshot.device_counts.is_empty() {
        // stable ordering: by device name
        let mut devices: Vec<_> = snapshot.device_counts.iter().collect();
        devices.sort_by_key(|(k, _)| *k);

        out.push_str("Devices: ");
        for (d, c) in devices {
            out.push_str(&format!("{d}={c} "));
        }
        out.push('\n');
    }

    // Identity semantics: these are counts of events with connection type info present.
    if !snapshot.connection_type_counts.is_empty() {
        let mut connection_types: Vec<_> = snapshot.connection_type_counts.iter().collect();
        connection_types.sort_by_key(|(k, _)| *k);
        out.push_str("Connection types: ");
        for (connection_type, c) in connection_types {
            out.push_str(&format!("{connection_type}={c} "));
        }
        out.push('\n');
    }

    // Identity semantics: these are counts of events with country info present.
    if !snapshot.country_counts.is_empty() {
        let mut countries: Vec<_> = snapshot.country_counts.iter().collect();
        countries.sort_by_key(|(k, _)| *k);
        out.push_str("Countries: ");
        for (country, c) in countries {
            out.push_str(&format!("{country}={c} "));
        }
        out.push('\n');
    }

    // Identity semantics: these are counts of events with ASN info present.
    if !snapshot.asn_counts.is_empty() {
        let mut asns: Vec<_> = snapshot.asn_counts.iter().collect();
        asns.sort_by_key(|(k, _)| *k);
        out.push_str("ASNs: ");
        for (asn, c) in asns {
            out.push_str(&format!("{asn}={c} "));
        }
        out.push('\n');
    }

    // Identity semantics: these are counts of events with ASO info present.
    if !snapshot.aso_counts.is_empty() {
        let mut asos: Vec<_> = snapshot.aso_counts.iter().collect();
        asos.sort_by_key(|(k, _)| *k);
        out.push_str("ASOs: ");
        for (aso, c) in asos {
            out.push_str(&format!("\n  {aso}={c}"));
        }
        out.push('\n');
    }
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
