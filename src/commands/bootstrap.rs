use anyhow::{Result, Context, bail};
use std::path::Path;
use std::fs;
use crate::commands::{run_cmd, run_cmd_stdin, read_cluster_metadata};
use crate::output;
use crate::paths;
use crate::types::ClusterConfig;

pub fn validate_bootstrap(cluster_dir: &str) -> Result<bool> {
    match generate_bootstrap_config(cluster_dir) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

pub fn generate_bootstrap_config(cluster_dir: &str) -> Result<String> {
    validate_inputs(cluster_dir)?;
    let metadata = extract_cluster_metadata(cluster_dir)?;
    generate_bootstrap(&metadata, cluster_dir, None)
}

pub fn execute(cluster_dir: &str, deploy: bool, admin_prep: bool, registry: Option<String>) -> Result<()> {
    output::header("Bootstrap");

    validate_inputs(cluster_dir)?;
    let metadata = extract_cluster_metadata(cluster_dir)?;

    output::summary(&metadata.cluster_name);
    output::config(
        "flux",
        &format!("{} (ns: {})", metadata.flux.install, metadata.flux.namespace),
    );

    if admin_prep {
        output::section("Generating admin-prep manifest");
        let admin_yaml = generate_admin_prep(&metadata, cluster_dir, registry)?;
        println!("{}", admin_yaml);
        output::done("Admin-prep manifest generated — hand this to the cluster admin");
        return Ok(());
    }

    output::section("Generating configuration");
    let bootstrap_yaml = generate_bootstrap(&metadata, cluster_dir, registry)?;
    println!("{}", bootstrap_yaml);

    if deploy {
        output::section("Deploying");
        deploy_bootstrap(&bootstrap_yaml)?;
        output::done("Bootstrap complete");
    }

    Ok(())
}

fn resolve_component_namespaces(cluster_dir: &str) -> Vec<String> {
    use crate::commands::matrix;

    let entries = match matrix::discover_cluster_artifacts(cluster_dir) {
        Ok(e) => e,
        Err(e) => {
            output::item_warn(&format!("failed to discover components: {}", e));
            return Vec::new();
        }
    };

    let mut namespaces = std::collections::BTreeSet::new();
    for entry in &entries {
        output::detail(&format!("building {} to resolve namespaces", entry.target_name));
        match crate::commands::build::build_single_artifact(entry, false) {
            Ok(rendered) => {
                for doc in rendered.split("\n---") {
                    for line in doc.lines() {
                        let trimmed = line.trim();
                        if let Some(ns) = trimmed.strip_prefix("namespace:") {
                            let ns = ns.trim().trim_matches('"').trim_matches('\'');
                            if !ns.is_empty() {
                                namespaces.insert(ns.to_string());
                            }
                        }
                    }
                }
            }
            Err(e) => {
                output::item_warn(&format!("failed to build {}: {}", entry.target_name, e));
            }
        }
    }
    namespaces.into_iter().collect()
}

const TPL_HEADER: &str = include_str!("../templates/admin-prep-header.yaml");
const TPL_NAMESPACE: &str = include_str!("../templates/admin-prep-namespace.yaml");
const TPL_SERVICE_ACCOUNT: &str = include_str!("../templates/admin-prep-serviceaccount.yaml");
const TPL_SOURCE_RBAC: &str = include_str!("../templates/admin-prep-source-controller-rbac.yaml");
const TPL_KUSTOMIZE_RBAC: &str = include_str!("../templates/admin-prep-kustomize-controller-rbac.yaml");
const TPL_DEPLOYER_RBAC: &str = include_str!("../templates/admin-prep-deployer-rbac.yaml");

fn generate_admin_prep(
    metadata: &ClusterConfig,
    cluster_dir: &str,
    registry: Option<String>,
) -> Result<String> {
    let ns = &metadata.flux.namespace;
    let target_ns = resolve_component_namespaces(cluster_dir);

    let oci_registry = registry
        .or_else(|| std::env::var("OCI_REGISTRY").ok())
        .context("--registry or OCI_REGISTRY required to generate Flux CRDs")?;
    let flux_registry = format!("{}/fluxcd", oci_registry);

    output::detail("Extracting Flux CRDs via flux install --export");
    let ns_arg = format!("--namespace={}", ns);
    let reg_arg = format!("--registry={}", flux_registry);
    let raw = run_cmd(
        "flux",
        &[
            "install",
            "--export",
            &ns_arg,
            "--components=source-controller,kustomize-controller",
            &reg_arg,
            "--image-pull-secret=image-pull-secret",
        ],
    )?;
    let raw_str = String::from_utf8_lossy(&raw);
    let crds = extract_crds(&raw_str);

    let mut out = String::new();

    out.push_str(&TPL_HEADER.replace("{cluster_name}", &metadata.cluster_name));
    out.push_str(&crds);
    out.push_str(&TPL_NAMESPACE.replace("{ns}", ns));

    for sa in &["source-controller", "kustomize-controller"] {
        out.push_str(&TPL_SERVICE_ACCOUNT.replace("{sa}", sa).replace("{ns}", ns));
    }

    out.push_str(&TPL_SOURCE_RBAC.replace("{ns}", ns));
    out.push_str(&TPL_KUSTOMIZE_RBAC.replace("{ns}", ns));

    for target in &target_ns {
        out.push_str(&TPL_DEPLOYER_RBAC.replace("{target}", target).replace("{ns}", ns));
    }

    if target_ns.is_empty() {
        output::item_warn("No enabled components with namespaces found — RBAC only covers flux namespace");
    }

    Ok(out)
}

fn extract_crds(yaml: &str) -> String {
    let mut crds = String::new();
    let mut in_crd = false;
    for line in yaml.lines() {
        if line == "---" {
            if in_crd {
                crds.push_str(line);
                crds.push('\n');
            }
            in_crd = false;
            continue;
        }
        if line.starts_with("kind: CustomResourceDefinition") {
            in_crd = true;
            crds.push_str("---\n");
        }
        if in_crd {
            crds.push_str(line);
            crds.push('\n');
        }
    }
    crds
}

fn validate_inputs(cluster_dir: &str) -> Result<()> {
    if !Path::new(cluster_dir).is_dir() {
        bail!("cluster directory not found at {}", cluster_dir);
    }
    if !Path::new(&format!("{}/cluster.yaml", cluster_dir)).exists() {
        bail!(
            "cluster.yaml not found at {}/cluster.yaml",
            cluster_dir
        );
    }
    if !Path::new(paths::CLUSTER_SCHEMA).exists() {
        bail!("cluster schema not found at {}", paths::CLUSTER_SCHEMA);
    }
    Ok(())
}

fn extract_cluster_metadata(cluster_dir: &str) -> Result<ClusterConfig> {
    let cluster_file = format!("{}/cluster.yaml", cluster_dir);
    read_cluster_metadata(Path::new(&cluster_file))
}

fn collect_component_overlays(metadata: &ClusterConfig) -> Vec<String> {
    use crate::commands::matrix::find_component_path;

    let mut overlays = Vec::new();
    for component in &metadata.components {
        let path = match find_component_path(&component.name) {
            Some(p) => p,
            None => continue,
        };
        let overlay_path = format!("{}/overlay.yaml", path);
        if Path::new(&overlay_path).exists() {
            output::detail(&format!("Including overlay from {}", component.id()));
            overlays.push(overlay_path);
        }
    }
    overlays
}

fn generate_bootstrap(
    metadata: &ClusterConfig,
    cluster_dir: &str,
    registry: Option<String>,
) -> Result<String> {
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();

    if metadata.flux.install {
        let exclude_kinds: Vec<String> = metadata.flux.exclude_kinds.iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect();

        if exclude_kinds.is_empty() {
            output::detail("Including Flux controllers (full install)");
        } else {
            output::detail(&format!("Including Flux controllers (excluding: {})", exclude_kinds.join(", ")));
        }

        let oci_registry = registry
            .or_else(|| std::env::var("OCI_REGISTRY").ok())
            .context("--registry or OCI_REGISTRY required for Flux installation")?;
        let flux_registry = format!("{}/fluxcd", oci_registry);

        let ns_arg = format!("--namespace={}", metadata.flux.namespace);
        let reg_arg = format!("--registry={}", flux_registry);
        let stdout = run_cmd(
            "flux",
            &[
                "install",
                "--export",
                &ns_arg,
                "--components-extra=image-reflector-controller,image-automation-controller",
                &reg_arg,
                "--image-pull-secret=image-pull-secret",
            ],
        )?;

        let flux_yaml = if exclude_kinds.is_empty() {
            stdout
        } else {
            strip_resource_kinds(&String::from_utf8_lossy(&stdout), &exclude_kinds).into_bytes()
        };

        fs::write(temp_path.join("flux-install.yaml"), flux_yaml)
            .context("Failed to write flux install manifest")?;
    }

    let mut ytt_args = vec![
        "-f".to_string(),
        paths::CLUSTER_SCHEMA.to_string(),
        "-f".to_string(),
        format!("{}/cluster.yaml", cluster_dir),
    ];

    for overlay_path in collect_component_overlays(metadata) {
        ytt_args.push("-f".to_string());
        ytt_args.push(overlay_path);
    }

    let flux_install_path = temp_path.join("flux-install.yaml");
    if metadata.flux.install && flux_install_path.exists() {
        ytt_args.push("-f".to_string());
        ytt_args.push(flux_install_path.to_string_lossy().to_string());
    }

    if Path::new(paths::BOOTSTRAP_SOURCES_BASE).is_dir() {
        ytt_args.push("-f".to_string());
        ytt_args.push(paths::BOOTSTRAP_SOURCES_BASE.to_string());
    }

    let cluster_overlay = format!("{}/overlays", cluster_dir);
    if Path::new(&cluster_overlay).is_dir() {
        output::detail(&format!(
            "Including overlays from {}",
            Path::new(cluster_dir)
                .file_name()
                .unwrap()
                .to_string_lossy()
        ));
        ytt_args.push("-f".to_string());
        ytt_args.push(cluster_overlay);
    }

    let ytt_args_str: Vec<&str> = ytt_args.iter().map(|s| s.as_str()).collect();
    let stdout = run_cmd("ytt", &ytt_args_str)?;

    let bootstrap_yaml = String::from_utf8_lossy(&stdout).to_string();
    Ok(substitute_secrets(&bootstrap_yaml))
}

fn strip_resource_kinds(yaml: &str, exclude_kinds: &[String]) -> String {
    let mut out = String::new();
    let mut current_doc = String::new();
    let mut skip = false;

    for line in yaml.lines() {
        if line == "---" {
            if !skip && !current_doc.is_empty() {
                out.push_str("---\n");
                out.push_str(&current_doc);
            }
            current_doc.clear();
            skip = false;
            continue;
        }

        if line.starts_with("kind: ") {
            let kind = line.trim_start_matches("kind: ").trim();
            if exclude_kinds.iter().any(|k| k == kind) {
                skip = true;
            }
        }

        current_doc.push_str(line);
        current_doc.push('\n');
    }

    if !skip && !current_doc.is_empty() {
        out.push_str("---\n");
        out.push_str(&current_doc);
    }

    out
}

fn substitute_secrets(yaml: &str) -> String {
    let env_vars = [
        "OCI_DOCKERCONFIG_JSON",
        "SPLUNK_HEC_HOST",
        "SPLUNK_HEC_TOKEN",
    ];
    let mut result = yaml.to_string();
    for var in env_vars {
        let placeholder = format!("${{{}}}", var);
        let value = std::env::var(var).unwrap_or_default();
        result = result.replace(&placeholder, &value);
    }
    result
}

fn deploy_bootstrap(yaml: &str) -> Result<()> {
    run_cmd("kubectl", &["cluster-info"])?;
    output::item_ok("kubectl configured");

    run_cmd_stdin("kubectl", &["apply", "-f", "-"], yaml.as_bytes())?;

    output::item_ok("Configuration applied");
    Ok(())
}
