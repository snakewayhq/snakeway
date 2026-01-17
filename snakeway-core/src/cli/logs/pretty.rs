use crate::cli::logs::types::LogEvent;

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
