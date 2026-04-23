pub mod bootstrap;
pub mod build;
pub mod compare;
pub mod explain;
pub mod inspect;
pub mod helm_manage;
pub mod init;
pub mod matrix;
pub mod validate;
pub mod mirror_flux;
pub mod status;

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use anyhow::{Result, Context, bail};
use crate::output;
use crate::types::ClusterConfig;

pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

pub fn command_exists(command: &str) -> bool {
    Command::new(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|mut c| { let _ = c.kill(); true })
        .unwrap_or(false)
}

pub fn run_command_stdout(command: &str, args: &[&str]) -> Option<String> {
    Command::new(command)
        .args(args)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

pub fn run_cmd(cmd: &str, args: &[&str]) -> Result<Vec<u8>> {
    output::cmd(cmd, args);
    let result = Command::new(cmd)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute {}", cmd))?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        bail!("{} failed: {}", cmd, stderr.trim());
    }
    Ok(result.stdout)
}

pub fn run_cmd_stdin(cmd: &str, args: &[&str], input: &[u8]) -> Result<Vec<u8>> {
    output::cmd(cmd, args);
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start {}", cmd))?;
    child.stdin.take().unwrap().write_all(input)?;
    let result = child
        .wait_with_output()
        .with_context(|| format!("failed to run {}", cmd))?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        bail!("{} failed: {}", cmd, stderr.trim());
    }
    Ok(result.stdout)
}

pub fn read_cluster_metadata(cluster_file: &Path) -> Result<ClusterConfig> {
    let file_str = cluster_file.to_string_lossy();
    let stdout = run_cmd(
        "ytt",
        &["-f", &file_str, "--data-values-inspect", "-o", "json"],
    )?;
    serde_json::from_slice(&stdout).context("failed to parse cluster metadata")
}

pub fn get_current_context() -> Result<String> {
    let stdout = run_cmd("kubectl", &["config", "current-context"])?;
    Ok(String::from_utf8_lossy(&stdout).trim().to_string())
}

pub struct RegistryCredentials {
    pub url: String,
    pub username: String,
    pub password: String,
}

pub fn resolve_registry_credentials(
    registry: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> Result<RegistryCredentials> {
    Ok(RegistryCredentials {
        url: registry
            .or_else(|| std::env::var("OCI_REGISTRY").ok())
            .context("Registry URL required (--registry or OCI_REGISTRY env var)")?,
        username: username
            .or_else(|| std::env::var("OCI_REGISTRY_USER").ok())
            .context("Registry username required (--registry-user or OCI_REGISTRY_USER env var)")?,
        password: password
            .or_else(|| std::env::var("OCI_REGISTRY_PASS").ok())
            .context("Registry password required (--registry-pass or OCI_REGISTRY_PASS env var)")?,
    })
}

pub struct TaskFailure {
    pub name: String,
    pub error: Option<String>,
}

impl TaskFailure {
    pub fn new(name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            error: Some(error.into()),
        }
    }
}

pub fn report_failures(failures: &[TaskFailure]) {
    for f in failures {
        match &f.error {
            Some(err) => output::item_fail(&format!("{}: {}", f.name, err)),
            None => output::item_fail(&f.name),
        }
    }
}
