use anyhow::Result;
use std::path::Path;
use walkdir::WalkDir;
use crate::commands::{read_cluster_metadata, normalize_path};
use crate::output;
use crate::paths;
use crate::types::{BuildMatrix, BuildMatrixEntry, ClusterMatrixEntry};

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
        let cluster_file = cluster_dir.join("cluster.yaml");

        if !cluster_file.exists() {
            continue;
        }

        let config = read_cluster_metadata(&cluster_file)?;

        for component in &config.components {
            if let Some(path) = find_component_path(&component.name) {
                build_matrix.push(BuildMatrixEntry {
                    artifact_path: path,
                    cluster: normalize_path(&cluster_dir.to_string_lossy()),
                    cluster_name: config.cluster_name.clone(),
                    target_name: format!("{}-{}", component.id(), config.cluster_name),
                    component_id: component.id().to_string(),
                    component_namespace: component.namespace().to_string(),
                    helm_flags: component.helm_flags.clone(),
                });
            }
        }

        cluster_matrix.push(ClusterMatrixEntry {
            cluster: normalize_path(&cluster_dir.to_string_lossy()),
            cluster_name: config.cluster_name.clone(),
        });
    }

    Ok(BuildMatrix {
        build_matrix,
        cluster_matrix,
    })
}

pub fn discover_cluster_artifacts(cluster_dir: &str) -> Result<Vec<BuildMatrixEntry>> {
    let normalized = normalize_path(cluster_dir);
    let normalized = normalized.trim_end_matches('/');
    let full_matrix = discover_matrix()?;
    Ok(full_matrix
        .build_matrix
        .into_iter()
        .filter(|a| a.cluster == normalized)
        .collect())
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

    output::done(&format!(
        "{} artifacts, {} clusters",
        result.build_matrix.len(),
        result.cluster_matrix.len()
    ));

    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}

pub fn find_component_path(component_name: &str) -> Option<String> {
    let components_path = format!("{}/{}", paths::COMPONENTS_DIR, component_name);
    let rgds_path = format!("{}/{}", paths::RGDS_DIR, component_name);

    if Path::new(&components_path).is_dir()
        && (Path::new(&format!("{}/base", components_path)).is_dir()
            || Path::new(&format!("{}/component.yaml", components_path)).exists())
    {
        return Some(components_path);
    }

    if Path::new(&rgds_path).is_dir()
        && (Path::new(&format!("{}/base", rgds_path)).is_dir()
            || Path::new(&format!("{}/component.yaml", rgds_path)).exists())
    {
        return Some(rgds_path);
    }

    None
}
