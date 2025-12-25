use anyhow::Result;
use std::path::PathBuf;

pub fn init(path: PathBuf) -> Result<()> {
    use anyhow::{Context, bail};
    use std::fs;
    use std::io::Write;

    // Refuse to overwrite an existing non-empty directory
    if path.exists() {
        if path.is_file() {
            bail!("{} exists and is not a directory", path.display());
        }

        let is_empty = fs::read_dir(&path)?.next().is_none();
        if !is_empty {
            bail!(
                "config directory '{}' already exists and is not empty",
                path.display()
            );
        }
    }

    // Create directory structure
    let routes_dir = path.join("routes");
    let services_dir = path.join("services");
    let devices_dir = path.join("devices");

    fs::create_dir_all(&routes_dir)?;
    fs::create_dir_all(&services_dir)?;
    fs::create_dir_all(&devices_dir)?;

    // Helper to write a file atomically-ish
    fn write_file(path: &PathBuf, contents: &str) -> Result<()> {
        let mut f = fs::File::create(path)
            .with_context(|| format!("failed to create {}", path.display()))?;
        f.write_all(contents.trim_start().as_bytes())?;
        Ok(())
    }

    // Write files
    write_file(&path.join("snakeway.toml"), ENTRYPOINT_TOML)?;
    write_file(&routes_dir.join("default.toml"), ROUTES_TOML)?;
    write_file(&services_dir.join("api.toml"), SERVICES_TOML)?;
    write_file(&devices_dir.join("identity.toml"), DEVICE_IDENTITY_TOML)?;
    write_file(
        &devices_dir.join("structured_logging.toml"),
        DEVICE_LOGGING_TOML,
    )?;

    // User feedback
    println!("✔ Initialized Snakeway config in {}", path.display());
    println!("✔ Created:");
    println!("  - snakeway.toml");
    println!("  - routes/default.toml");
    println!("  - services/api.toml");
    println!("  - devices/identity.toml");
    println!("  - devices/structured_logging.toml");

    println!();
    println!("Next steps:");
    println!("  snakeway config check");
    println!("  snakeway run");

    Ok(())
}

const ENTRYPOINT_TOML: &str = r#"
[server]
listen = "0.0.0.0:8080"
threads = 4
pid_file = "/var/run/snakeway.pid"

[include]
routes   = "routes/*.toml"
services = "services/*.toml"
devices  = "devices/*.toml"
"#;

const ROUTES_TOML: &str = r#"
[[route]]
path = "/api"
service = "api"

[[route]]
path = "/"
file_dir = "./public"
index = "index.html"
directory_listing = false
"#;

const SERVICES_TOML: &str = r#"
[[service]]
name = "api"
strategy = "round_robin"

[[service.upstream]]
url = "http://127.0.0.1:9001"

[[service.upstream]]
url = "http://127.0.0.1:9002"
"#;

const DEVICE_IDENTITY_TOML: &str = r#"
[[device]]
name = "identity"
type = "builtin"
builtin = "identity"

[device.config]
enable_geoip = false
trusted_proxies = ["10.0.0.0/8"]
user_agent = "woothee"
"#;

const DEVICE_LOGGING_TOML: &str = r#"
[[device]]
name = "structured_logging"
type = "builtin"
builtin = "structured_logging"

[device.config]
format = "json"
include_headers = true
"#;
