use anyhow::{Context, Result};
use rust_embed::RustEmbed;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(RustEmbed)]
#[folder = "config-templates/"]
pub struct ConfigTemplates;

pub fn init(path: PathBuf) -> Result<()> {
    use anyhow::bail;

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

    // Write files...

    // Entrypoint
    write_file(&path.join("snakeway.toml"), &template("snakeway.toml")?)?;

    // Routes
    write_file(&routes_dir.join("api.toml"), &template("routes/api.toml")?)?;
    write_file(&routes_dir.join("ws.toml"), &template("routes/ws.toml")?)?;
    write_file(
        &routes_dir.join("assets.toml"),
        &template("routes/assets.toml")?,
    )?;

    // Services
    write_file(
        &services_dir.join("api.toml"),
        &template("services/api.toml")?,
    )?;

    // Devices
    write_file(
        &devices_dir.join("identity.toml"),
        &template("devices/identity.toml")?,
    )?;
    write_file(
        &devices_dir.join("structured_logging.toml"),
        &template("devices/structured_logging.toml")?,
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

/// Fetch an embedded config template as UTF-8 text
fn template(path: &str) -> Result<String> {
    let file = ConfigTemplates::get(path)
        .with_context(|| format!("missing embedded config template: {path}"))?;

    let s =
        std::str::from_utf8(file.data.as_ref()).context("config template is not valid UTF-8")?;

    Ok(s.to_owned())
}

/// Helper to write a file (simple, deterministic, no magic)
fn write_file(path: &PathBuf, contents: &str) -> Result<()> {
    let mut f =
        fs::File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    f.write_all(contents.trim_start().as_bytes())?;
    Ok(())
}
