use clap::ValueEnum;
use snakeway_conf::load_config;
use snakeway_conf::validation::{ConfigError, render_json, render_plain, render_pretty};
use std::path::PathBuf;

pub(crate) fn check(
    path: PathBuf,
    quiet: bool,
    format: ConfigCheckOutputFormat,
) -> anyhow::Result<()> {
    match load_config(&path) {
        Ok(cfg) => {
            if quiet {
                return Ok(());
            }

            if matches!(format, ConfigCheckOutputFormat::Json) {
                let success_info = serde_json::json!({
                    "status": "success",
                    "routes": cfg.routes.len(),
                    "services": cfg.services.len(),
                    "upstreams": cfg.services.len(),
                    "devices_enabled": cfg.devices.iter().filter(|d| d.is_enabled()).count()
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&success_info).expect("could not format JSON")
                );
            } else {
                println!("Config loaded successfully");
                println!("{} routes", cfg.routes.len());
                println!("{} services", cfg.services.len());
                println!("{} upstreams", cfg.services.len());
                println!(
                    "{} devices enabled",
                    cfg.devices.iter().filter(|d| d.is_enabled()).count()
                );
            }
            Ok(())
        }
        Err(err) => {
            if quiet {
                std::process::exit(1);
            }

            match err {
                ConfigError::SemanticValidationFailed { validation_report } => {
                    match format {
                        ConfigCheckOutputFormat::Pretty => {
                            render_pretty(&validation_report);
                        }
                        ConfigCheckOutputFormat::Plain => {
                            render_plain(&validation_report);
                        }
                        ConfigCheckOutputFormat::Json => {
                            render_json(&validation_report);
                        }
                    };
                }
                _ => {
                    eprintln!("{}", err);
                }
            }

            std::process::exit(1);
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub(crate) enum ConfigCheckOutputFormat {
    Pretty,
    Plain,
    Json,
}
