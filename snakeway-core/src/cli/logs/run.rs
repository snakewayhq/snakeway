use crate::cli::logs::constants::{LOOP_IDLE_SLEEP, RENDER_TICK, WINDOW};
use crate::cli::logs::parse::parse_event;
use crate::cli::logs::pretty::render_pretty;
use crate::cli::logs::stats_aggregation::StatsAggregator;
use crate::cli::logs::stats_rendering::{redraw, render_stats};
use crate::cli::logs::types::LogEvent;
use crate::logging::LogMode;
use anyhow::Result;
use serde_json::Value;
use std::io::{self, BufRead, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

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
