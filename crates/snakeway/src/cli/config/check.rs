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
            if validated.has_warnings() {
                let mut out = String::new();
                validated.render_plain(&mut out);
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
                ConfigError::SemanticValidationFailed { report, sources } => {
                    match format {
                        ConfigCheckOutputFormat::Pretty => {
                            let mut out = String::new();
                            report.render_pretty(&sources, &mut out).ok();
                            eprint!("{out}");
                        }
                        ConfigCheckOutputFormat::Plain => {
                            let mut out = String::new();
                            report.render_plain(&sources, &mut out).ok();
                            eprint!("{out}");
                        }
                        ConfigCheckOutputFormat::Json => {
                            let mut out = String::new();
                            report.render_json(&sources, &mut out).ok();
                            eprintln!("{out}");
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
