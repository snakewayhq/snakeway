use crate::conf::load_config;
use std::path::PathBuf;
use std::str::FromStr;

pub fn check(path: PathBuf, quiet: bool, format: ConfigCheckOutputFormat) -> anyhow::Result<()> {
    match load_config(&path) {
        Ok(validation_cfg) => {
            let cfg = validation_cfg.config;
            let validation_report = validation_cfg.validation_report;

            // Validation...
            if validation_report.has_violations() {
                if !quiet {
                    match format {
                        ConfigCheckOutputFormat::Pretty => {
                            validation_report.render_pretty();
                        }
                        ConfigCheckOutputFormat::Plain => {
                            validation_report.render_plain();
                        }
                        ConfigCheckOutputFormat::Json => {
                            validation_report.render_json();
                        }
                    };
                }
                std::process::exit(1);
            }

            // Success...
            if quiet {
                // Print nothing.
            } else if matches!(format, ConfigCheckOutputFormat::Json) {
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
                println!("✔ Config loaded successfully");
                println!("✔ {} routes", cfg.routes.len());
                println!("✔ {} services", cfg.services.len());
                println!("✔ {} upstreams", cfg.services.len());
                println!(
                    "✔ {} devices enabled",
                    cfg.devices.iter().filter(|d| d.is_enabled()).count()
                );
            }
            Ok(())
        }
        Err(err) => {
            if !quiet {
                match format {
                    ConfigCheckOutputFormat::Pretty => {
                        eprintln!("{}", err);
                    }
                    ConfigCheckOutputFormat::Plain => {
                        eprintln!("{}", err);
                    }
                    ConfigCheckOutputFormat::Json => {
                        eprintln!("{}", err);
                    }
                }
            }

            std::process::exit(1);
        }
    }
}

#[derive(Clone, Debug)]
pub enum ConfigCheckOutputFormat {
    Pretty,
    Plain,
    Json,
}

impl FromStr for ConfigCheckOutputFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pretty" => Ok(Self::Pretty),
            "plain" => Ok(Self::Plain),
            "json" => Ok(Self::Json),
            _ => Err(anyhow::anyhow!("invalid output format: {}", s)),
        }
    }
}
