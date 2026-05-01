use anyhow::Result;
use crate::commands::run_cmd_stdin;

pub fn resolve_image_digests(yaml_content: &str) -> Result<String> {
    let stdout = run_cmd_stdin("kbld", &["-f", "-"], yaml_content.as_bytes())?;
    Ok(String::from_utf8_lossy(&stdout).to_string())
}
