use crate::conf::load_config;
use std::path::PathBuf;

pub fn check(path: PathBuf) -> anyhow::Result<()> {
    let cfg = load_config(&path)?;

    println!("✔ Config loaded successfully");
    println!("✔ {} routes", cfg.routes.len());
    println!("✔ {} services", cfg.services.len());
    println!(
        "✔ {} upstreams",
        cfg.services
            .values()
            .map(|s| s.upstream.len())
            .sum::<usize>()
    );
    println!(
        "✔ {} devices enabled",
        cfg.devices.iter().filter(|d| d.enabled).count()
    );

    Ok(())
}
