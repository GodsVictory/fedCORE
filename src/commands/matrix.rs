use anyhow::{Result, Context, bail};
use std::path::Path;
use walkdir::WalkDir;
use crate::commands::{run_cmd, normalize_path, resolve_cluster_yaml};
use crate::output;
use crate::paths;
use crate::types::{BuildMatrix, BuildMatrixEntry, ClusterMatrixEntry};

fn resolve_cluster(cluster_dir: &str, cluster_yaml: &str) -> Result<(String, Vec<String>, Vec<BuildMatrixEntry>)> {
    let stdout = run_cmd(
        "ytt",
        &["-f", paths::CLUSTER_SCHEMA, "-f", cluster_yaml, "-f", paths::BUILD_MATRIX_DIR],
    )?;
    let stdout_str = String::from_utf8_lossy(&stdout);
    let mut docs = stdout_str.split("\n---").map(|s| s.trim()).filter(|s| !s.is_empty());

    let header: serde_json::Value = serde_saphyr::from_str(
        docs.next().context("empty matrix output")?,
    ).context("failed to parse cluster header")?;

    let cluster_name = header["cluster_name"].as_str().context("missing cluster_name")?.to_string();
    let overlays: Vec<String> = header["overlays"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();

    let mut entries = Vec::new();
    for doc in docs {
        let val: serde_json::Value = serde_saphyr::from_str(doc)
            .context("failed to parse component entry")?;
        let c = &val["component"];
        let name = c["name"].as_str().context("missing component name")?;
        let id = c["id"].as_str().context("missing component id")?;

        if let Some(artifact_path) = find_component_path(name) {
            entries.push(BuildMatrixEntry {
                artifact_path,
                cluster: cluster_dir.to_string(),
                cluster_name: cluster_name.clone(),
                target_name: format!("{}-{}", id, cluster_name),
                component_id: id.to_string(),
                component_namespace: c["namespace"].as_str().unwrap_or(id).to_string(),
                component_data_values_yaml: doc.to_string(),
                overlays: overlays.clone(),
            });
        }
    }

    Ok((cluster_name, overlays, entries))
}

pub fn discover_matrix() -> Result<BuildMatrix> {
    let mut build_matrix = Vec::new();
    let mut cluster_matrix = Vec::new();

    for entry in WalkDir::new(paths::CLUSTERS_DIR)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let cluster_dir = entry.path();
        if !cluster_dir.join("cluster.yaml").exists() {
            continue;
        }

        let dir_str = normalize_path(&cluster_dir.to_string_lossy());
        let temp = tempfile::tempdir()?;
        let cluster_yaml = resolve_cluster_yaml(&dir_str, temp.path())?;
        let (cluster_name, _, entries) = resolve_cluster(&dir_str, &cluster_yaml)?;

        build_matrix.extend(entries);
        cluster_matrix.push(ClusterMatrixEntry {
            cluster: dir_str,
            cluster_name,
        });
    }

    Ok(BuildMatrix { build_matrix, cluster_matrix })
}

pub fn discover_cluster_artifacts(cluster_dir: &str) -> Result<Vec<BuildMatrixEntry>> {
    let normalized = normalize_path(cluster_dir);
    let normalized = normalized.trim_end_matches('/');
    if !Path::new(&normalized).join("cluster.yaml").exists() {
        bail!("cluster.yaml not found in {}", normalized);
    }
    let temp = tempfile::tempdir()?;
    let cluster_yaml = resolve_cluster_yaml(normalized, temp.path())?;
    let (_, _, entries) = resolve_cluster(normalized, &cluster_yaml)?;
    Ok(entries)
}

pub fn execute() -> Result<()> {
    output::header("Matrix");
    let result = discover_matrix()?;

    output::section("OCI Artifacts");
    for item in &result.build_matrix {
        output::item_ok(&item.target_name);
    }
    output::section("Clusters");
    for item in &result.cluster_matrix {
        output::item_ok(&item.cluster_name);
    }
    output::done(&format!("{} artifacts, {} clusters", result.build_matrix.len(), result.cluster_matrix.len()));
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn find_component_path(component_name: &str) -> Option<String> {
    for base in [paths::COMPONENTS_DIR, paths::RGDS_DIR] {
        let path = format!("{}/{}", base, component_name);
        if Path::new(&path).is_dir()
            && (Path::new(&format!("{}/base", path)).is_dir()
                || Path::new(&format!("{}/component.yaml", path)).exists())
        {
            return Some(path);
        }
    }
    None
}
