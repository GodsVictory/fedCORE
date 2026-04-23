use anyhow::Result;
use rayon::prelude::*;
use std::path::Path;
use std::sync::Mutex;
use walkdir::WalkDir;

use crate::commands::{self, build, bootstrap, TaskFailure, report_failures};
use crate::output;
use crate::paths;

pub fn execute() -> Result<()> {
    output::header("Validate");

    let mut validation_failed = false;

    output::section("Tools");
    validation_failed |= !check_tools()?;

    output::section("Schema");
    validation_failed |= !validate_cluster_schema()?;

    output::section("Components");
    validation_failed |= !validate_components_via_build()?;

    output::section("Clusters");
    validation_failed |= !validate_clusters_via_bootstrap()?;

    if validation_failed {
        output::fail("Validation failed");
        anyhow::bail!("Validation failed with errors");
    } else {
        output::done("All validations passed");
    }

    Ok(())
}

fn check_tools() -> Result<bool> {
    let mut all_ok = true;

    if commands::command_exists("ytt") {
        let version = get_tool_version("ytt", &["version"])?;
        output::item_ok(&format!("ytt ({})", version));
    } else {
        output::item_fail("ytt not installed");
        output::detail("https://carvel.dev/ytt/");
        all_ok = false;
    }

    if commands::command_exists("flux") {
        let version = get_tool_version("flux", &["version", "--client"])?;
        output::item_ok(&format!(
            "flux ({})",
            version.lines().next().unwrap_or("unknown")
        ));
    } else {
        output::item_fail("flux not installed");
        output::detail("https://fluxcd.io/");
        all_ok = false;
    }

    if commands::command_exists("helm") {
        let version = get_tool_version("helm", &["version", "--short"])?;
        output::item_ok(&format!("helm ({})", version.trim()));
    } else {
        output::item_fail("helm not installed");
        output::detail("https://helm.sh/");
        all_ok = false;
    }

    if commands::command_exists("kubectl") {
        output::item_ok("kubectl");
    } else {
        output::item_warn("kubectl not installed (required for deploy)");
    }

    Ok(all_ok)
}

fn validate_cluster_schema() -> Result<bool> {
    if !Path::new(paths::CLUSTER_SCHEMA).exists() {
        output::item_fail(&format!("Schema not found: {}", paths::CLUSTER_SCHEMA));
        return Ok(false);
    }

    match commands::run_cmd("ytt", &["-f", paths::CLUSTER_SCHEMA, "--data-values-inspect"]) {
        Ok(_) => {
            output::item_ok("Cluster schema valid");
            Ok(true)
        }
        Err(_) => {
            output::item_fail("Cluster schema invalid");
            Ok(false)
        }
    }
}

fn discover_all_clusters() -> Vec<String> {
    let mut clusters = Vec::new();

    for entry in WalkDir::new(paths::CLUSTERS_DIR)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let path = entry.path();
        if path.join("cluster.yaml").exists() {
            clusters.push(path.to_string_lossy().to_string());
        }
    }

    clusters.sort();
    clusters
}

fn validate_components_via_build() -> Result<bool> {
    use crate::commands::matrix;

    let full_matrix = matrix::discover_matrix()?;
    let artifacts = &full_matrix.build_matrix;

    if artifacts.is_empty() {
        output::item_warn("No component-cluster combinations found");
        return Ok(true);
    }

    let total = artifacts.len();
    let cluster_count = discover_all_clusters().len();
    let component_count = artifacts
        .iter()
        .map(|a| a.artifact_path.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();
    output::summary(&format!(
        "{} combinations ({} components x {} clusters)",
        total, component_count, cluster_count
    ));

    let pb = output::progress_bar(total as u64);
    let failures = Mutex::new(Vec::<TaskFailure>::new());

    artifacts.par_iter().for_each(|artifact| {
        let component_name = Path::new(&artifact.artifact_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        pb.set_message(format!("{} @ {}", component_name, artifact.cluster_name));

        match build::validate_build(&artifact.artifact_path, &artifact.cluster) {
            Ok(true) => {}
            Ok(false) => {
                failures.lock().unwrap().push(TaskFailure {
                    name: artifact.target_name.clone(),
                    error: None,
                });
            }
            Err(e) => {
                failures.lock().unwrap().push(TaskFailure::new(
                    &artifact.target_name,
                    format!("{}", e),
                ));
            }
        }

        pb.inc(1);
    });

    pb.finish_and_clear();

    let failures = failures.into_inner().unwrap();
    let failed_count = failures.len();
    let passed_count = total - failed_count;

    if failures.is_empty() && passed_count > 0 {
        output::item_ok(&format!("{} combinations valid", passed_count));
        Ok(true)
    } else if passed_count == 0 && failures.is_empty() {
        output::item_warn("No components tested");
        Ok(true)
    } else {
        output::item_fail(&format!(
            "{} builds failed, {} passed",
            failed_count, passed_count
        ));
        for (i, failure) in failures.iter().enumerate().take(10) {
            report_failures(std::slice::from_ref(failure));
            if i == 9 && failed_count > 10 {
                output::item_fail(&format!("... and {} more", failed_count - 10));
                break;
            }
        }
        Ok(false)
    }
}

fn validate_clusters_via_bootstrap() -> Result<bool> {
    let clusters = discover_all_clusters();

    if clusters.is_empty() {
        output::item_warn("No clusters found");
        return Ok(true);
    }

    output::summary(&format!("{} clusters", clusters.len()));

    let pb = output::progress_bar(clusters.len() as u64);
    let failures = Mutex::new(Vec::<TaskFailure>::new());

    clusters.par_iter().for_each(|cluster_dir| {
        let cluster_name = Path::new(cluster_dir)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        pb.set_message(cluster_name.clone());

        match bootstrap::validate_bootstrap(cluster_dir) {
            Ok(true) => {}
            Ok(false) => {
                failures.lock().unwrap().push(TaskFailure {
                    name: cluster_name,
                    error: None,
                });
            }
            Err(e) => {
                failures.lock().unwrap().push(TaskFailure::new(
                    cluster_name,
                    format!("{}", e),
                ));
            }
        }

        pb.inc(1);
    });

    pb.finish_and_clear();

    let failures = failures.into_inner().unwrap();
    let failed_count = failures.len();
    let passed_count = clusters.len() - failed_count;

    if failures.is_empty() && passed_count > 0 {
        output::item_ok(&format!("{} clusters valid", passed_count));
        Ok(true)
    } else if passed_count == 0 && failures.is_empty() {
        output::item_warn("No clusters tested");
        Ok(true)
    } else {
        output::item_fail(&format!(
            "{} cluster bootstraps failed",
            failed_count
        ));
        report_failures(&failures);
        Ok(false)
    }
}

fn get_tool_version(command: &str, args: &[&str]) -> Result<String> {
    commands::run_command_stdout(command, args)
        .ok_or_else(|| anyhow::anyhow!("Failed to get version for {}", command))
}
