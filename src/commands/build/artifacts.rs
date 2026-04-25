use anyhow::{Result, bail};
use rayon::prelude::*;
use std::path::Path;
use std::sync::Mutex;
use std::fs;
use crate::output;
use crate::paths;
use crate::types::{BuildMatrixEntry, MergedComponent};
use crate::commands::{read_cluster_metadata, TaskFailure, report_failures};
use super::{utils::*, overlays::*, rendering::*};

pub fn build_single_artifact(
    entry: &BuildMatrixEntry,
    save_to_dist: bool,
) -> Result<String> {
    if !Path::new(&entry.artifact_path).is_dir() {
        bail!("artifact directory not found at {}", entry.artifact_path);
    }
    if !Path::new(&entry.cluster).is_dir() {
        bail!("cluster directory not found at {}", entry.cluster);
    }

    let cluster_file = format!("{}/cluster.yaml", entry.cluster);
    if !Path::new(&cluster_file).exists() {
        bail!("cluster.yaml not found at {}", cluster_file);
    }

    let cluster_data = read_cluster_metadata(Path::new(&cluster_file))?;

    output::detail(&format!(
        "{} for {}",
        entry.component_id, cluster_data.cluster_name
    ));

    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();

    let (pre_render, post_render) =
        collect_overlays(&entry.artifact_path, &cluster_data.overlays)?;
    let (platform_pre, platform_post) = collect_platform_overlays()?;

    let component_file = format!("{}/component.yaml", entry.artifact_path);
    let manifests_path;

    if Path::new(&component_file).exists() {
        apply_prerender_overlays(&component_file, &cluster_file, temp_path, &pre_render, &platform_pre)?;

        let component_data: MergedComponent = serde_yaml::from_str(
            &fs::read_to_string(temp_path.join("component-merged.yaml"))?,
        )?;

        if component_data.helm.is_some() {
            output::detail("type: helm chart");
            render_helm_chart(temp_path, &entry.component_id, &entry.component_namespace, entry.helm_flags.as_deref())?;
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

    let base_dir = format!("{}/base", entry.artifact_path);
    if Path::new(&base_dir).is_dir() {
        render_base_manifests(&cluster_file, &base_dir, &manifests_path)?;
        output::detail("base manifests rendered");
    }

    let post_overlay_content =
        apply_postrender_overlays(&manifests_path, &cluster_file, &entry.cluster, &post_render, &platform_post)?;

    output::detail("resolving image tags to digests");
    let output_content = resolve_image_digests(&post_overlay_content)?;

    if save_to_dist {
        fs::create_dir_all(paths::DIST_DIR)?;
        let output_file = format!("{}/{}.yaml", paths::DIST_DIR, entry.target_name);
        fs::write(&output_file, &output_content)?;
        validate_yaml(&output_file)?;
        output::detail(&format!("wrote {}", output_file));
    }

    Ok(output_content)
}

pub fn build_artifacts(entries: &[BuildMatrixEntry]) -> Result<()> {
    if entries.is_empty() {
        bail!("No components to build");
    }

    let artifact_count = entries.len();
    fs::create_dir_all(paths::DIST_DIR)?;

    let pb = output::progress_bar(artifact_count as u64);
    let failures = Mutex::new(Vec::<TaskFailure>::new());
    let built = Mutex::new(Vec::<String>::new());

    entries.par_iter().for_each(|artifact| {
        if let Err(e) = build_single_artifact(artifact, true) {
            failures.lock().unwrap().push(TaskFailure::new(
                &artifact.target_name,
                format!("{}", e),
            ));
        } else {
            built.lock().unwrap().push(format!(
                "{}/{}.yaml",
                paths::DIST_DIR,
                artifact.target_name
            ));
        }

        pb.set_message(artifact.component_id.clone());
        pb.inc(1);
    });

    pb.finish_and_clear();

    let failures = failures.into_inner().unwrap();
    let mut built = built.into_inner().unwrap();
    built.sort();
    for path in &built {
        output::item_ok(path);
    }
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

    Ok(())
}
