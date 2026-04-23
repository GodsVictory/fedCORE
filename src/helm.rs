use anyhow::{Result, Context, bail};
use std::path::Path;
use std::process::Command;
use std::fs;
use crate::output;
use crate::paths::HELM_CACHE_DIR;

pub fn cached_path(chart: &str, version: &str) -> String {
    format!("{}/{}-{}.tgz", HELM_CACHE_DIR, chart, version)
}

pub fn is_cached(chart: &str, version: &str) -> bool {
    Path::new(&cached_path(chart, version)).exists()
}

pub fn resolve_cached_chart(chart: &str, version: &str, repo_url: &str) -> Result<String> {
    let filename = format!("{}-{}.tgz", chart, version);
    let cache_path = format!("{}/{}", HELM_CACHE_DIR, filename);

    if Path::new(&cache_path).exists() {
        return Ok(cache_path);
    }

    fs::create_dir_all(HELM_CACHE_DIR)?;

    output::cmd("helm", &["pull", repo_url, "--version", version, "--destination", HELM_CACHE_DIR]);
    let result = Command::new("helm")
        .args(["pull", repo_url, "--version", version, "--destination", HELM_CACHE_DIR])
        .output()
        .context("Failed to pull helm chart")?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        bail!("Failed to download chart {}:{}: {}", chart, version, stderr);
    }

    Ok(cache_path)
}
