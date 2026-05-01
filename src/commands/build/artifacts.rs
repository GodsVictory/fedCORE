use anyhow::{Result, bail};
use rayon::prelude::*;
use std::path::Path;
use std::sync::Mutex;
use std::fs;
use crate::output;
use crate::paths;
use crate::types::{BuildMatrixEntry, MergedComponent};
use crate::commands::{TaskFailure, report_failures, resolve_cluster_yaml};
use super::{utils::*, overlays::*, rendering::render_helm_chart};

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

    output::detail(&format!(
        "{} for {}",
        entry.component_id, entry.cluster_name
    ));

    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();

    let cluster_file = resolve_cluster_yaml(&entry.cluster, temp_path)?;

    let data_values_file = temp_path.join("component-values.yaml");
    fs::write(&data_values_file, &entry.component_data_values_yaml)?;
    let data_values_path = data_values_file.to_string_lossy().to_string();

    let (pre_render_dirs, post_render_dirs) =
        collect_overlay_dirs(&entry.artifact_path, &entry.overlays);

    let component_file = format!("{}/component.yaml", entry.artifact_path);
    let manifests_path;

    if Path::new(&component_file).exists() {
        apply_prerender_overlays(&component_file, &cluster_file, temp_path, &pre_render_dirs, &data_values_path)?;

        let merged_path = temp_path.join("component-merged.yaml");
        let component_data: MergedComponent = serde_saphyr::from_str(
            &fs::read_to_string(&merged_path)?,
        )?;

        if let Some(ref helm) = component_data.helm {
            let helm_namespace = helm.namespace.as_deref()
                .unwrap_or(&entry.component_namespace);
            output::detail("type: helm chart");
            render_helm_chart(temp_path, helm, &entry.component_id, helm_namespace)?;
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
    let base_dir_opt = if Path::new(&base_dir).is_dir() {
        Some(base_dir.as_str())
    } else {
        None
    };

    let post_overlay_content =
        apply_postrender_overlays(&manifests_path, &cluster_file, &entry.cluster, base_dir_opt, &post_render_dirs, &data_values_path)?;

    output::detail("resolving image tags to digests");
    let output_content = resolve_image_digests(&post_overlay_content)?;

    if save_to_dist {
        fs::create_dir_all(paths::DIST_DIR)?;
        let output_file = format!("{}/{}.yaml", paths::DIST_DIR, entry.target_name);
        fs::write(&output_file, &output_content)?;
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
