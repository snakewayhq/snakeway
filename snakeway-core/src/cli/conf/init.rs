use anyhow::{Context, Result};
use rust_embed::RustEmbed;
use std::fs;
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

    // Map embedded templates to their destination paths
    let mut created_files = Vec::new();
    for template_path in ConfigTemplates::iter() {
        let template_path = template_path.as_ref();
        let file = ConfigTemplates::get(template_path)
            .with_context(|| format!("missing embedded template: {template_path}"))?;

        // Destination path matches template path (keeping .d suffixes)
        let dest_rel_path = template_path.to_string();
        let dest_path = path.join(&dest_rel_path);

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }

        let contents = std::str::from_utf8(file.data.as_ref())
            .context("config template is not valid UTF-8")?
            .trim_start();

        fs::write(&dest_path, contents.as_bytes())
            .with_context(|| format!("failed to write {}", dest_path.display()))?;

        created_files.push(dest_rel_path);
    }

    // Sort for deterministic output
    created_files.sort();

    // User feedback
    println!("✔ Initialized Snakeway config in {}", path.display());
    println!("✔ Created:");
    for file in created_files {
        println!("  - {file}");
    }
    println!();
    println!("Next steps:");
    println!("  snakeway config check");
    println!("  snakeway run");

    Ok(())
}
