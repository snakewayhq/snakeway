use crate::conf::error::ConfigError;
use crate::conf::load_config;
use std::path::PathBuf;

pub fn check(path: PathBuf) -> anyhow::Result<()> {
    match load_config(&path) {
        Ok(cfg) => {
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

        Err(err) => {
            print_config_error(&err);
            Err(err.into())
        }
    }
}

fn print_config_error(err: &ConfigError) {
    eprintln!("error: {}", err);
}
