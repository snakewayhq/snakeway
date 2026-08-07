use super::stats_aggregation::StatsSnapshot;
use crate::cli::logs::types::LogEvent;
use std::io;
use std::io::Write;

pub(crate) fn render_stats(snapshot: &StatsSnapshot) -> String {
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

pub(crate) fn redraw(output: &str) {
    print!("\x1b[2J\x1b[H");
    println!("{output}");
    let _ = io::stdout().flush();
}

pub(crate) fn render_pretty(event: &LogEvent) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn snapshot() -> StatsSnapshot {
        StatsSnapshot {
            window_seconds: 60,
            rps: 2.5,
            window_events: 4,
            latency: vec![("0–10ms".to_string(), 3), (">10ms".to_string(), 1)],
            status: (1, 2, 3),
            p95_ms: 42,
            p99_ms: 99,
            device_counts: HashMap::from([("mac".to_string(), 2), ("phone".to_string(), 1)]),
            connection_type_counts: HashMap::from([("cellular".to_string(), 1)]),
            asn_counts: HashMap::from([(64512, 1)]),
            aso_counts: HashMap::from([("TestNet".to_string(), 1)]),
            country_counts: HashMap::from([("NZ".to_string(), 1)]),
            bot_count: 6,
            human_count: 5,
            unknown_identity_count: 7,
        }
    }

    #[test]
    fn should_render_stats_headline_and_sections() {
        // Arrange
        let snapshot = snapshot();

        // Act
        let out = render_stats(&snapshot);

        // Assert
        assert!(out.contains("(60s window)"), "out: {out}");
        assert!(out.contains("RPS: 2.5 | events: 4 | 5xx: 3"), "out: {out}");
        assert!(
            out.contains("75.0%"),
            "latency percentages must render: {out}"
        );
        assert!(
            out.contains("25.0%"),
            "latency percentages must render: {out}"
        );
        assert!(
            out.contains(&"█".repeat(15)),
            "a 75 percent bucket renders 15 bars: {out}"
        );
        assert!(
            !out.contains(&"█".repeat(16)),
            "no bucket renders more bars than its percentage: {out}"
        );
        assert!(out.contains("p95 ≈ 42ms | p99 ≈ 99ms"), "out: {out}");
        assert!(out.contains("2xx=1 4xx=2 5xx=3"), "out: {out}");
        assert!(out.contains("human=5 bot=6 unknown=7"), "out: {out}");
        assert!(
            out.contains("Devices: mac=2 phone=1"),
            "devices must render sorted by name: {out}"
        );
        assert!(out.contains("Connection types: cellular=1"), "out: {out}");
        assert!(out.contains("Countries: NZ=1"), "out: {out}");
        assert!(out.contains("ASNs: 64512=1"), "out: {out}");
        assert!(out.contains("TestNet=1"), "out: {out}");
    }

    #[test]
    fn should_render_a_placeholder_without_latency_samples() {
        // Arrange
        let snapshot = StatsSnapshot {
            latency: vec![("0–10ms".to_string(), 0)],
            device_counts: HashMap::new(),
            connection_type_counts: HashMap::new(),
            asn_counts: HashMap::new(),
            aso_counts: HashMap::new(),
            country_counts: HashMap::new(),
            ..snapshot()
        };

        // Act
        let out = render_stats(&snapshot);

        // Assert
        assert!(out.contains("Latency (window): <no samples>"), "out: {out}");
        assert!(
            !out.contains("Devices:"),
            "empty sections must not render: {out}"
        );
    }
}
