use anyhow::Result;
use std::path::Path;
use std::fs;
use crate::commands::run_cmd;
use crate::output;
use crate::paths;

pub fn collect_overlay_dirs(
    artifact_path: &str,
    overlays: &[String],
) -> (Vec<String>, Vec<String>) {
    let mut pre_render = Vec::new();
    let mut post_render = Vec::new();

    for id in overlays {
        let pre = format!("{}/overlays/{}/pre-render", artifact_path, id);
        if Path::new(&pre).is_dir() {
            pre_render.push(pre);
        }

        let post = format!("{}/overlays/{}/post-render", artifact_path, id);
        if Path::new(&post).is_dir() {
            post_render.push(post);
        }
    }

    (pre_render, post_render)
}

pub fn apply_prerender_overlays(
    component_file: &str,
    cluster_file: &str,
    temp_dir: &Path,
    overlay_dirs: &[String],
    data_values_path: &str,
) -> Result<()> {
    let mut args = vec![
        "-f".to_string(),
        paths::CLUSTER_SCHEMA.to_string(),
        "-f".to_string(),
        cluster_file.to_string(),
        "-f".to_string(),
        component_file.to_string(),
        "-f".to_string(),
        paths::BUILD_PRERENDER_DIR.to_string(),
        "--data-values-file".to_string(),
        data_values_path.to_string(),
    ];

    if !overlay_dirs.is_empty() {
        output::detail(&format!("{} pre-render overlay dir(s)", overlay_dirs.len()));
        for dir in overlay_dirs {
            args.push("-f".to_string());
            args.push(dir.clone());
        }
    }

    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let stdout = run_cmd("ytt", &args_str)?;

    fs::write(temp_dir.join("component-merged.yaml"), stdout)?;
    Ok(())
}

pub fn apply_postrender_overlays(
    manifests_path: &Path,
    cluster_file: &str,
    cluster_dir: &str,
    base_dir: Option<&str>,
    overlay_dirs: &[String],
    data_values_path: &str,
) -> Result<String> {
    let mut args = vec![
        "--ignore-unknown-comments".to_string(),
        "-f".to_string(),
        paths::CLUSTER_SCHEMA.to_string(),
        "-f".to_string(),
        cluster_file.to_string(),
        "-f".to_string(),
        manifests_path.to_string_lossy().to_string(),
        "--data-values-file".to_string(),
        data_values_path.to_string(),
    ];

    if let Some(base) = base_dir {
        output::detail("including base manifests");
        args.push("-f".to_string());
        args.push(base.to_string());
    }

    if !overlay_dirs.is_empty() {
        output::detail(&format!("{} post-render overlay dir(s)", overlay_dirs.len()));
        for dir in overlay_dirs {
            args.push("-f".to_string());
            args.push(dir.clone());
        }
    }

    let cluster_overlay_dir = format!("{}/overlays", cluster_dir);
    if Path::new(&cluster_overlay_dir).is_dir() {
        let cluster_name = Path::new(cluster_dir)
            .file_name()
            .unwrap()
            .to_string_lossy();
        output::detail(&format!("cluster overlays from {}", cluster_name));
        args.push("-f".to_string());
        args.push(cluster_overlay_dir);
    }

    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let stdout = run_cmd("ytt", &args_str)?;

    Ok(String::from_utf8_lossy(&stdout).to_string())
}
