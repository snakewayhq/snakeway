use std::fs;
use std::path::Path;
use tempfile::{NamedTempFile, TempPath};

pub fn render_config(template: &str, listen_port: u16, upstream_port: u16) -> TempPath {
    let template_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(template);

    let contents = fs::read_to_string(&template_path)
        .unwrap_or_else(|e| panic!("failed to read config template {:?}: {}", template_path, e));

    let rendered = contents
        .replace("{LISTEN_ADDR}", &format!("127.0.0.1:{listen_port}"))
        .replace("{UPSTREAM_ADDR}", &format!("127.0.0.1:{upstream_port}"));

    let file = NamedTempFile::new().expect("failed to create temp config file");

    fs::write(file.path(), rendered).expect("failed to write rendered config");

    // IMPORTANT: return TempPath so file lives as long as owner
    file.into_temp_path()
}
