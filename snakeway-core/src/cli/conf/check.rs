use crate::conf::load_config;
use crate::conf::validation::error::ConfigError;
use miette::{JSONReportHandler, Report};
use std::io::{self, Write};
use std::path::PathBuf;
use std::str::FromStr;

pub fn check(path: PathBuf, quiet: bool, format: ConfigCheckOutputFormat) -> anyhow::Result<()> {
    match load_config(&path) {
        Ok(cfg) => {
            if quiet {
                // Print nothing.
            } else if matches!(format, ConfigCheckOutputFormat::Json) {
                let success_info = serde_json::json!({
                    "status": "success",
                    "routes": cfg.routes.len(),
                    "services": cfg.services.len(),
                    "upstreams": cfg.services
                        .values()
                        .map(|s| s.upstream.len())
                        .sum::<usize>(),
                    "devices_enabled": cfg.devices.iter().filter(|d| d.enabled).count()
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&success_info).expect("could not format JSON")
                );
            } else {
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
            }
            Ok(())
        }
        Err(err) => {
            if !quiet {
                print_config_error(err, format);
            }

            std::process::exit(1);
        }
    }
}

fn print_config_error(err: ConfigError, format: ConfigCheckOutputFormat) {
    match format {
        ConfigCheckOutputFormat::Pretty => {
            let hint = config_error_hint(&err);

            eprintln!("{:?}", Report::new(err));

            if let Some(hint) = hint {
                eprintln!("\nHint:\n{hint}");
            }
        }

        ConfigCheckOutputFormat::Plain => {
            eprintln!("{}", err);
        }

        ConfigCheckOutputFormat::Json => {
            let mut out = String::new();

            let handler = JSONReportHandler::new();
            handler
                .render_report(&mut out, &err)
                .expect("failed to render JSON diagnostic");

            let mut stdout = io::stdout();
            stdout.write_all(out.as_bytes()).unwrap();
            stdout.write_all(b"\n").unwrap();
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

pub fn config_error_hint(err: &ConfigError) -> Option<&'static str> {
    match err {
        //---------------------------------------------------------------------
        // Route structure / intent errors
        //---------------------------------------------------------------------
        ConfigError::MissingDirForStaticRoute { .. } => Some(
            "Static routes must declare a filesystem directory.\n\
             \n\
             Example:\n\
             \n\
             [[route]]\n\
             type = \"static\"\n\
             path = \"/static\"\n\
             file_dir  = \"/var/www\"",
        ),

        ConfigError::MissingServiceForServiceRoute { .. } => Some(
            "Service routes must reference a named upstream service.\n\
             \n\
             Example:\n\
             \n\
             [[route]]\n\
             type    = \"service\"\n\
             path    = \"/api\"\n\
             service = \"api\"",
        ),

        ConfigError::ServiceNotAllowedOnStaticRoute { .. } => Some(
            "Static routes serve files from disk and must not reference services.\n\
             \n\
             Remove the `service` field or change the route type to `service`.",
        ),

        ConfigError::DirNotAllowedOnServiceRoute { .. } => Some(
            "Service routes forward requests to upstream services and must not define `dir`.\n\
             \n\
             Remove the `dir` field or change the route type to `static`.",
        ),

        ConfigError::WebSocketNotAllowedOnStaticRoute { .. } => Some(
            "WebSockets are only supported on service routes.\n\
             \n\
             To use WebSockets, change the route type to `service` and ensure the upstream supports them.",
        ),

        //---------------------------------------------------------------------
        // Listener errors
        //---------------------------------------------------------------------
        ConfigError::Http2RequiresTls => Some(
            "HTTP/2 requires TLS.\n\
             \n\
             Enable TLS on the listener or disable HTTP/2.",
        ),

        ConfigError::AdminListenerMissingTls => Some(
            "Admin listeners must use TLS.\n\
             \n\
             Configure a certificate and key for the admin listener.",
        ),

        ConfigError::AdminListenerHttp2NotSupported => Some(
            "The admin interface does not support HTTP/2.\n\
             \n\
             Disable HTTP/2 on the admin listener.",
        ),

        ConfigError::MultipleAdminListeners => Some(
            "Only one admin listener may be defined.\n\
             \n\
             Remove or disable additional admin listeners.",
        ),

        //---------------------------------------------------------------------
        // Service / upstream errors
        //---------------------------------------------------------------------
        ConfigError::EmptyService { .. } => Some(
            "Services must define at least one upstream.\n\
             \n\
             Example:\n\
             \n\
             [[service]]\n\
             name = \"api\"\n\
             [[service.upstream]]\n\
             url = \"http://127.0.0.1:8080\"",
        ),

        ConfigError::InvalidUpstream { .. } => Some(
            "Upstream URLs must use supported schemes (http or https) and have a positive weight.\n\
             \n\
             Example:\n\
             \n\
             [[service.upstream]]\n\
             url    = \"http://127.0.0.1:8080\"\n\
             weight = 1",
        ),

        ConfigError::InvalidCircuitBreaker { .. } => Some(
            "All circuit breaker thresholds must be greater than zero.\n\
             \n\
             Review the circuit breaker configuration for this service.",
        ),

        //---------------------------------------------------------------------
        // Everything else: no hint
        //---------------------------------------------------------------------
        _ => None,
    }
}
