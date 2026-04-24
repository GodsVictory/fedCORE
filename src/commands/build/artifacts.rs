use anyhow::{Result, bail};
use rayon::prelude::*;
use std::path::Path;
use std::sync::Mutex;
use std::fs;
use crate::output;
use crate::paths;
use crate::types::{BuildMatrix, BuildMatrixEntry, MergedComponent};
use crate::commands::{read_cluster_metadata, TaskFailure, report_failures};
use super::{utils::*, overlays::*, rendering::*};

pub fn build_single_artifact(
    artifact_path: &str,
    cluster_dir: &str,
    save_to_dist: bool,
) -> Result<String> {
    if !Path::new(artifact_path).is_dir() {
        bail!("artifact directory not found at {}", artifact_path);
    }
    if !Path::new(cluster_dir).is_dir() {
        bail!("cluster directory not found at {}", cluster_dir);
    }

    let cluster_file = format!("{}/cluster.yaml", cluster_dir);
    if !Path::new(&cluster_file).exists() {
        bail!("cluster.yaml not found at {}", cluster_file);
    }

    let cluster_data = read_cluster_metadata(Path::new(&cluster_file))?;
    let artifact_name = Path::new(artifact_path)
        .file_name()
        .unwrap()
        .to_string_lossy();

    output::detail(&format!(
        "{} for {}",
        artifact_name, cluster_data.cluster_name
    ));

    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();

    let (pre_render, post_render) =
        collect_overlays(artifact_path, &cluster_data.overlays)?;
    let (platform_pre, platform_post) = collect_platform_overlays()?;

    let component_file = format!("{}/component.yaml", artifact_path);
    let manifests_path;

    if Path::new(&component_file).exists() {
        apply_prerender_overlays(&component_file, &cluster_file, temp_path, &pre_render, &platform_pre)?;

        let component_data: MergedComponent = serde_yaml::from_str(
            &fs::read_to_string(temp_path.join("component-merged.yaml"))?,
        )?;

        if component_data.helm.is_some() {
            output::detail("type: helm chart");
            render_helm_chart(temp_path)?;
            manifests_path = temp_path.join("helm-rendered.yaml");
        } else {
            output::detail("type: plain manifests");
            manifests_path = temp_path.join("plain-rendered.yaml");
            fs::write(&manifests_path, "")?;
        }
    } else {
        manifests_path = temp_path.join("plain-rendered.yaml");
        fs::write(&manifests_path, "")?;
    }

    let base_dir = format!("{}/base", artifact_path);
    if Path::new(&base_dir).is_dir() {
        render_base_manifests(&cluster_file, &base_dir, &manifests_path)?;
        output::detail("base manifests rendered");
    }

    let post_overlay_content =
        apply_postrender_overlays(&manifests_path, &cluster_file, cluster_dir, &post_render, &platform_post)?;

    output::detail("resolving image tags to digests");
    let output_content = resolve_image_digests(&post_overlay_content)?;

    if save_to_dist {
        fs::create_dir_all(paths::DIST_DIR)?;
        let target_name = format!("{}-{}", artifact_name, cluster_data.cluster_name);
        let output_file = format!("{}/{}.yaml", paths::DIST_DIR, target_name);
        fs::write(&output_file, &output_content)?;
        validate_yaml(&output_file)?;
        output::detail(&format!("wrote {}", output_file));
    }

    Ok(output_content)
}

pub fn build_cluster_artifacts(cluster_dir: &str) -> Result<Vec<BuildMatrixEntry>> {
    use crate::commands::matrix;

    let cluster_artifacts = matrix::discover_cluster_artifacts(cluster_dir)?;

    if cluster_artifacts.is_empty() {
        bail!("No components found for cluster: {}", cluster_dir);
    }

    let artifact_count = cluster_artifacts.len();
    let cluster_name = get_cluster_name(cluster_dir)?;

    output::summary(&format!(
        "{} components for {}",
        artifact_count, cluster_name
    ));
    fs::create_dir_all(paths::DIST_DIR)?;

    let pb = output::progress_bar(artifact_count as u64);
    let failures = Mutex::new(Vec::<TaskFailure>::new());

    cluster_artifacts.par_iter().for_each(|artifact| {
        let component_name = Path::new(&artifact.artifact_path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        pb.set_message(component_name);

        if let Err(e) = build_single_artifact(&artifact.artifact_path, &artifact.cluster, true) {
            failures.lock().unwrap().push(TaskFailure::new(
                &artifact.target_name,
                format!("{}", e),
            ));
        }

        pb.inc(1);
    });

    pb.finish_and_clear();

    let failures = failures.into_inner().unwrap();
    if failures.is_empty() {
        output::done(&format!(
            "Built {} components for {}",
            artifact_count, cluster_name
        ));
    } else {
        report_failures(&failures);
        output::fail(&format!(
            "{}/{} failed",
            failures.len(),
            artifact_count
        ));
        bail!("Build failed");
    }

    Ok(cluster_artifacts)
}

pub fn build_all_artifacts() -> Result<BuildMatrix> {
    use crate::commands::matrix;

    let matrix = matrix::discover_matrix()?;
    let artifact_count = matrix.build_matrix.len();

    output::summary(&format!("{} artifacts", artifact_count));
    fs::create_dir_all(paths::DIST_DIR)?;

    let pb = output::progress_bar(artifact_count as u64);
    let failures = Mutex::new(Vec::<TaskFailure>::new());

    matrix.build_matrix.par_iter().for_each(|artifact| {
        pb.set_message(artifact.target_name.clone());

        if let Err(e) = build_single_artifact(&artifact.artifact_path, &artifact.cluster, true) {
            failures.lock().unwrap().push(TaskFailure::new(
                &artifact.target_name,
                format!("{}", e),
            ));
        }

        pb.inc(1);
    });

    pb.finish_and_clear();

    let failures = failures.into_inner().unwrap();
    if failures.is_empty() {
        output::done(&format!("Built {} artifacts", artifact_count));
    } else {
        report_failures(&failures);
        output::fail(&format!(
            "{}/{} failed",
            failures.len(),
            artifact_count
        ));
        bail!("Build failed");
    }

    Ok(matrix)
}
