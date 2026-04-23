use anyhow::{Result, Context, bail};
use std::path::Path;
use std::fs;
use walkdir::WalkDir;
use serde_json::Value;

use crate::output;
use crate::commands::{self, resolve_registry_credentials, run_cmd};

pub fn execute(
    target: &str,
    cluster: Option<String>,
    namespace: Option<String>,
    registry: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> Result<()> {
    output::header("Inspect");

    if target.contains("://") || target.contains('/') {
        output::section("Pulling artifact");
        let tmp_dir = resolve_oci(target, registry, username, password)?;
        display_path(tmp_dir.path())?;
        output::done("Inspect complete");
    } else {
        output::section("Resolving deployed component");
        let tmp_dir = resolve_deployed(
            target,
            cluster.as_deref(),
            namespace.as_deref(),
            username,
            password,
        )?;
        display_path(tmp_dir.path())?;
        output::done("Inspect complete");
    }

    Ok(())
}

pub(crate) fn resolve_target(
    target: &str,
    cluster: Option<&str>,
    namespace: Option<&str>,
    registry: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> Result<tempfile::TempDir> {
    if target.contains("://") || target.contains('/') {
        resolve_oci(target, registry, username, password)
    } else {
        resolve_deployed(target, cluster, namespace, username, password)
    }
}

fn resolve_oci(
    artifact: &str,
    registry: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> Result<tempfile::TempDir> {
    let creds = resolve_registry_credentials(registry, username, password)?;
    let oci_ref = if artifact.contains("://") {
        format!("oci://{}", artifact.trim_start_matches("oci://"))
    } else {
        format!("oci://{}/fedcore/{}", creds.url, artifact)
    };
    output::summary(&format!("Pulling {}", oci_ref));
    pull_artifact(&oci_ref, &creds.username, &creds.password)
}

fn resolve_deployed(
    component: &str,
    cluster: Option<&str>,
    namespace: Option<&str>,
    username: Option<String>,
    password: Option<String>,
) -> Result<tempfile::TempDir> {
    if !commands::command_exists("kubectl") {
        bail!("kubectl is not installed or not in PATH");
    }

    let context = match cluster {
        Some(c) => c.to_string(),
        None => commands::get_current_context()?,
    };

    output::summary(&format!("Resolving {} (context: {})", component, context));

    let label = format!("fedcore.io/component={}", component);
    let mut args = vec![
        "--context",
        &context,
        "--request-timeout=10s",
        "get",
        "ocirepository",
        "-l",
        &label,
        "-o",
        "json",
    ];
    if let Some(ns) = namespace {
        args.push("-n");
        args.push(ns);
    } else {
        args.push("-A");
    }

    let stdout = run_cmd("kubectl", &args)?;
    let json: Value = serde_json::from_slice(&stdout)?;
    let items = json["items"]
        .as_array()
        .context("Unexpected kubectl response format")?;

    let oci_repo = items
        .first()
        .with_context(|| format!("No OCIRepository found for component '{}'", component))?;

    let url = oci_repo["spec"]["url"]
        .as_str()
        .context("OCIRepository missing spec.url")?;

    let tag = oci_repo["spec"]["ref"]["tag"]
        .as_str()
        .map(String::from)
        .or_else(|| {
            oci_repo["status"]["artifact"]["revision"]
                .as_str()
                .and_then(|rev| rev.split('@').next())
                .filter(|t| !t.starts_with("sha256:"))
                .map(String::from)
        })
        .unwrap_or_else(|| "latest".to_string());

    let oci_ref = if url.starts_with("oci://") {
        format!("{}:{}", url, tag)
    } else {
        format!("oci://{}:{}", url, tag)
    };

    output::item_ok(&format!("Found: {}", oci_ref));

    let username = username
        .or_else(|| std::env::var("OCI_REGISTRY_USER").ok())
        .context("Registry username required for pulling (--registry-user or OCI_REGISTRY_USER)")?;
    let password = password
        .or_else(|| std::env::var("OCI_REGISTRY_PASS").ok())
        .context(
            "Registry password required for pulling (--registry-pass or OCI_REGISTRY_PASS)",
        )?;

    pull_artifact(&oci_ref, &username, &password)
}

fn pull_artifact(
    oci_ref: &str,
    username: &str,
    password: &str,
) -> Result<tempfile::TempDir> {
    let tmp_dir =
        tempfile::tempdir().context("Failed to create temporary directory")?;

    let creds_arg = format!("--creds={}:{}", username, password);
    let output_arg = format!("--output={}", tmp_dir.path().display());

    output::cmd(
        "flux",
        &["pull", "artifact", oci_ref, &output_arg, "--creds=***"],
    );
    let result = std::process::Command::new("flux")
        .args(["pull", "artifact", oci_ref, &output_arg, &creds_arg])
        .output()
        .context("Failed to execute flux CLI — is flux installed?")?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        output::fail("Pull failed");
        bail!("flux pull artifact failed: {}", stderr.trim());
    }

    output::item_ok("Downloaded");
    Ok(tmp_dir)
}

fn display_path(path: &Path) -> Result<()> {
    output::section("Contents");

    let mut found_files = false;
    for entry in WalkDir::new(path)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        found_files = true;
        let rel_path = entry.path().strip_prefix(path).unwrap_or(entry.path());
        let filename = rel_path.display().to_string();

        output::summary(&filename);

        let content = fs::read_to_string(entry.path())
            .with_context(|| format!("Failed to read {}", filename))?;

        println!("{}", content);
    }

    if !found_files {
        output::item_warn("No files found");
    }

    Ok(())
}
