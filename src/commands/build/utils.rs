use anyhow::Result;
use crate::commands::{run_cmd, run_cmd_stdin};
use crate::output;

pub fn resolve_image_digests(yaml_content: &str) -> Result<String> {
    let stdout = run_cmd_stdin("kbld", &["-f", "-"], yaml_content.as_bytes())?;
    Ok(String::from_utf8_lossy(&stdout).to_string())
}

pub fn validate_yaml(file_path: &str) -> Result<()> {
    if let Err(e) = run_cmd("ytt", &["--ignore-unknown-comments", "-f", file_path]) {
        output::item_fail(&format!("Validation failed: {}", e));
        anyhow::bail!("Validation failed");
    }
    Ok(())
}
