use anyhow::Result;
use rayon::prelude::*;
use std::process::Command;
use std::sync::{Mutex, Once};
use std::path::Path;
use std::fs;
use super::discovery::*;
use crate::helm;
use crate::output;
use crate::commands::{TaskFailure, report_failures};

fn ensure_chart(
    component: &ComponentInfo,
    dir: &str,
    repos_init: &Once,
    all_components: &[&ComponentInfo],
) -> Result<()> {
    let filename = format!("{}-{}.tgz", component.chart, component.version);
    let dest_path = format!("{}/{}", dir, filename);

    if Path::new(&dest_path).exists() {
        return Ok(());
    }

    if helm::is_cached(&component.chart, &component.version) {
        let cache_path = helm::cached_path(&component.chart, &component.version);
        fs::copy(&cache_path, &dest_path)?;
        return Ok(());
    }

    if !component.repo.starts_with("oci://") {
        repos_init.call_once(|| ensure_helm_repos(all_components));
    }

    let chart_ref = if component.repo.starts_with("oci://") {
        format!("{}/{}", component.repo, component.chart)
    } else {
        let repo_name = get_repo_name(&component.name);
        format!("{}/{}", repo_name, component.chart)
    };

    let cache_path =
        helm::resolve_cached_chart(&component.chart, &component.version, &chart_ref)?;
    fs::copy(&cache_path, &dest_path)?;
    Ok(())
}

pub fn download_current_versions(components: &[&ComponentInfo], dir: &str) -> Result<()> {
    output::section("Downloading charts");

    let repos_init = Once::new();
    let pb = output::progress_bar(components.len() as u64);
    let failures = Mutex::new(Vec::<TaskFailure>::new());

    components.par_iter().for_each(|component| {
        pb.set_message(format!("{}:{}", component.chart, component.version));

        if let Err(e) = ensure_chart(component, dir, &repos_init, components) {
            failures.lock().unwrap().push(TaskFailure::new(
                format!("{}:{}", component.chart, component.version),
                format!("{}", e),
            ));
        }

        pb.inc(1);
    });

    pb.finish_and_clear();

    let failures = failures.into_inner().unwrap();
    if !failures.is_empty() {
        report_failures(&failures);
        anyhow::bail!("{} chart downloads failed", failures.len());
    }

    Ok(())
}

pub fn discover_latest_versions(
    components: &[&ComponentInfo],
    dir: &str,
    update: bool,
) -> Result<()> {
    output::section("Discovering latest versions");

    ensure_helm_repos(components);

    let repos_init = Once::new();
    repos_init.call_once(|| {});
    let pb = output::progress_bar(components.len() as u64);
    let failures = Mutex::new(Vec::<TaskFailure>::new());

    components.par_iter().for_each(|component| {
        pb.set_message(component.name.clone());

        let latest_version = if component.repo.starts_with("oci://") {
            None
        } else {
            match get_latest_http_version(&component.name, &component.chart) {
                Ok(v) => v,
                Err(e) => {
                    failures.lock().unwrap().push(TaskFailure::new(
                        &component.name,
                        format!("{}", e),
                    ));
                    pb.inc(1);
                    return;
                }
            }
        };

        if let Some(version) = latest_version {
            let latest_component = ComponentInfo {
                name: component.name.clone(),
                chart: component.chart.clone(),
                repo: component.repo.clone(),
                version: version.clone(),
                component_path: component.component_path.clone(),
            };

            if let Err(e) = ensure_chart(&latest_component, dir, &repos_init, components) {
                failures.lock().unwrap().push(TaskFailure::new(
                    &component.name,
                    format!("{}", e),
                ));
            }

            if update {
                if let Err(e) = update_component_version(&component.component_path, &version) {
                    failures.lock().unwrap().push(TaskFailure::new(
                        &component.name,
                        format!("update failed: {}", e),
                    ));
                }
            }
        }

        pb.inc(1);
    });

    pb.finish_and_clear();

    let failures = failures.into_inner().unwrap();
    if !failures.is_empty() {
        report_failures(&failures);
        anyhow::bail!("{} version discoveries failed", failures.len());
    }

    Ok(())
}

fn ensure_helm_repos(components: &[&ComponentInfo]) {
    let has_http = components.iter().any(|c| !c.repo.starts_with("oci://"));
    if !has_http {
        return;
    }

    for component in components {
        if !component.repo.starts_with("oci://") {
            let repo_name = get_repo_name(&component.name);
            output::cmd("helm", &["repo", "add", &repo_name, &component.repo]);
            let _ = Command::new("helm")
                .args(["repo", "add", &repo_name, &component.repo])
                .output();
        }
    }

    output::cmd("helm", &["repo", "update"]);
    let _ = Command::new("helm").args(["repo", "update"]).output();
}

fn get_latest_http_version(name: &str, chart: &str) -> Result<Option<String>> {
    let repo_name = get_repo_name(name);
    let search_pattern = format!("{}/{}", repo_name, chart);

    output::cmd("helm", &["search", "repo", &search_pattern, "--versions"]);
    let output = Command::new("helm")
        .args(["search", "repo", &search_pattern, "--versions"])
        .output()
        .context("Failed to search helm repo")?;

    if !output.status.success() {
        return Ok(None);
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    for line in output_str.lines().skip(1) {
        if line.contains("DEPRECATED") {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            return Ok(Some(parts[1].to_string()));
        }
    }

    Ok(None)
}

fn update_component_version(component_path: &str, new_version: &str) -> Result<()> {
    let file_path = format!("platform/{}/component.yaml", component_path);
    let content = std::fs::read_to_string(&file_path)?;

    let mut in_helm = false;
    let mut replaced = false;
    let updated = content
        .lines()
        .map(|line| {
            if line == "helm:" || line.ends_with(" helm:") {
                in_helm = true;
            } else if in_helm && !line.starts_with(' ') && !line.is_empty() {
                in_helm = false;
            }
            if in_helm && !replaced && line.starts_with("  version:") {
                replaced = true;
                return format!("  version: \"{}\"", new_version);
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");

    std::fs::write(&file_path, updated)?;
    Ok(())
}

use anyhow::Context;
