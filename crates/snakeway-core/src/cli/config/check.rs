use clap::ValueEnum;
use snakeway_conf::load_config;
use snakeway_conf::validation::ConfigError;
use std::path::PathBuf;

pub(crate) fn check(
    path: PathBuf,
    quiet: bool,
    format: ConfigCheckOutputFormat,
) -> anyhow::Result<()> {
    match load_config(&path) {
        Ok(validated) => {
            if validated.report.has_warnings() {
                let mut out = String::new();
                validated.report.render_plain(&mut out).ok();
                eprint!("{out}");
            }

            if quiet {
                return Ok(());
            }

            let cfg = &validated.config;
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
                            let mut out = String::new();
                            validation_report.render_pretty(&mut out).ok();
                            eprint!("{out}");
                        }
                        ConfigCheckOutputFormat::Plain => {
                            let mut out = String::new();
                            validation_report.render_plain(&mut out).ok();
                            eprint!("{out}");
                        }
                        ConfigCheckOutputFormat::Json => {
                            validation_report.render_json(&mut std::io::stderr()).ok();
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
