use anyhow::Result;
use rayon::prelude::*;
use std::process::Command;
use std::sync::{Mutex, Once};
use std::path::Path;
use std::io::Read;
use std::fs;
use flate2::read::GzDecoder;
use tar::Archive;
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

    let cache_path =
        helm::resolve_cached_chart(&component.chart, &component.version, &component.chart_ref())?;
    fs::copy(&cache_path, &dest_path)?;
    Ok(())
}

pub fn download_current_versions(components: &[&ComponentInfo], dir: &str, update: bool) -> Result<()> {
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

        if update {
            if let Err(e) = copy_default_values(component) {
                failures.lock().unwrap().push(TaskFailure::new(
                    &component.name,
                    format!("copy default values failed: {}", e),
                ));
            }
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
                if let Err(e) = copy_default_values(&latest_component) {
                    failures.lock().unwrap().push(TaskFailure::new(
                        &component.name,
                        format!("copy default values failed: {}", e),
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

    let mut repo_names = Vec::new();
    for component in components {
        if !component.repo.starts_with("oci://") {
            let repo_name = get_repo_name(&component.name);
            output::cmd("helm", &["repo", "add", &repo_name, &component.repo]);
            let _ = Command::new("helm")
                .args(["repo", "add", &repo_name, &component.repo])
                .output();
            repo_names.push(repo_name);
        }
    }

    let args: Vec<&str> = std::iter::once("repo")
        .chain(std::iter::once("update"))
        .chain(repo_names.iter().map(|s| s.as_str()))
        .collect();

    output::cmd("helm", &args);
    let _ = Command::new("helm").args(&args).output();
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

fn copy_default_values(component: &ComponentInfo) -> Result<()> {
    let cache_path = helm::cached_path(&component.chart, &component.version);
    let tgz = fs::File::open(&cache_path)
        .context(format!("Could not open cached chart {}", cache_path))?;
    let decoder = GzDecoder::new(tgz);
    let mut archive = Archive::new(decoder);

    let values_suffix = "/values.yaml";
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().to_string();
        if path.ends_with(values_suffix) && path.matches('/').count() == 1 {
            let mut contents = String::new();
            entry.read_to_string(&mut contents)?;
            let dest = format!("platform/{}/default-values.yaml", component.component_path);
            fs::write(&dest, contents)?;
            return Ok(());
        }
    }

    anyhow::bail!("values.yaml not found in chart archive")
}

use anyhow::Context;
