use anyhow::{Result, Context};
use rayon::prelude::*;
use std::process::Command;
use std::sync::Mutex;
use std::path::Path;
use super::discovery::*;
use crate::commands::{resolve_registry_credentials, TaskFailure, report_failures};
use crate::output;

pub fn push_charts(
    components: &[&ComponentInfo],
    dir: &str,
    registry: Option<String>,
    registry_user: Option<String>,
    registry_pass: Option<String>,
) -> Result<()> {
    output::section("Pushing charts to OCI");

    let creds = resolve_registry_credentials(registry, registry_user, registry_pass)?;

    output::detail(&format!("logging in to {}", creds.url));

    output::cmd(
        "helm",
        &["registry", "login", &creds.url, "-u", &creds.username, "-p", "***"],
    );
    let login_output = Command::new("helm")
        .args([
            "registry",
            "login",
            &creds.url,
            "-u",
            &creds.username,
            "-p",
            &creds.password,
        ])
        .output()
        .context("Failed to login to OCI registry")?;

    if !login_output.status.success() {
        let stderr = String::from_utf8_lossy(&login_output.stderr);
        anyhow::bail!("Helm registry login failed: {}", stderr);
    }

    let pb = output::progress_bar(components.len() as u64);
    let failures = Mutex::new(Vec::<TaskFailure>::new());

    components.par_iter().for_each(|component| {
        let chart_file = format!(
            "{}/{}-{}.tgz",
            dir, component.chart, component.version
        );

        pb.set_message(format!("{}:{}", component.chart, component.version));

        if !Path::new(&chart_file).exists() {
            pb.inc(1);
            return;
        }

        let oci_url = format!(
            "oci://{}/fedcore/helm-charts",
            creds.url.trim_start_matches("oci://")
        );

        let chart_ref = format!(
            "{}/fedcore/helm-charts/{}:{}",
            creds.url.trim_start_matches("oci://"),
            component.chart,
            component.version
        );
        output::cmd(
            "helm",
            &[
                "pull",
                &format!("oci://{}", chart_ref),
                "--version",
                &component.version,
                "--destination",
                "/tmp",
            ],
        );
        let check_output = Command::new("helm")
            .args([
                "pull",
                &format!("oci://{}", chart_ref),
                "--version",
                &component.version,
                "--destination",
                "/tmp",
            ])
            .output();

        if let Ok(result) = check_output {
            if result.status.success() {
                pb.inc(1);
                return;
            }
        }

        output::cmd("helm", &["push", &chart_file, &oci_url]);
        let push_output = Command::new("helm")
            .args(["push", &chart_file, &oci_url])
            .output();

        match push_output {
            Ok(result) if !result.status.success() => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                if !stderr.contains("already exists") && !stderr.contains("manifest unknown") {
                    failures.lock().unwrap().push(TaskFailure::new(
                        format!("{}:{}", component.chart, component.version),
                        stderr.trim().to_string(),
                    ));
                }
            }
            Err(e) => {
                failures.lock().unwrap().push(TaskFailure::new(
                    format!("{}:{}", component.chart, component.version),
                    format!("{}", e),
                ));
            }
            _ => {}
        }

        pb.inc(1);
    });

    pb.finish_and_clear();

    output::cmd("helm", &["registry", "logout", &creds.url]);
    let _ = Command::new("helm")
        .args(["registry", "logout", &creds.url])
        .output();

    let failures = failures.into_inner().unwrap();
    if !failures.is_empty() {
        report_failures(&failures);
        anyhow::bail!("{} chart pushes failed", failures.len());
    }

    Ok(())
}
