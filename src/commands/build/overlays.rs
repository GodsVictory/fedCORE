use anyhow::Result;
use std::path::{Path, PathBuf};
use std::fs;
use walkdir::WalkDir;
use crate::commands::run_cmd;
use crate::output;
use crate::paths;

pub fn collect_overlays(
    artifact_path: &str,
    overlays: &[String],
) -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut pre_render = Vec::new();
    let mut post_render = Vec::new();

    for id in overlays {
        let dir = format!("{}/overlays/{}", artifact_path, id);
        if !Path::new(&dir).is_dir() {
            continue;
        }
        for entry in WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "yaml"))
        {
            let phase = get_overlay_phase(entry.path())?;
            if phase == "pre-render" {
                pre_render.push(entry.path().to_path_buf());
            } else {
                post_render.push(entry.path().to_path_buf());
            }
        }
    }

    Ok((pre_render, post_render))
}

pub fn collect_platform_overlays() -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut pre_render = Vec::new();
    let mut post_render = Vec::new();

    let dir = Path::new(paths::PLATFORM_OVERLAYS);
    if !dir.is_dir() {
        return Ok((pre_render, post_render));
    }

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "yaml"))
    {
        let phase = get_overlay_phase(entry.path())?;
        if phase == "pre-render" {
            pre_render.push(entry.path().to_path_buf());
        } else {
            post_render.push(entry.path().to_path_buf());
        }
    }

    Ok((pre_render, post_render))
}

fn get_overlay_phase(file_path: &Path) -> Result<String> {
    let content = fs::read_to_string(file_path)?;
    for line in content.lines() {
        if line.starts_with("#! overlay-phase:") {
            return Ok(line.replace("#! overlay-phase:", "").trim().to_string());
        }
    }
    Ok("post-render".to_string())
}

pub fn apply_prerender_overlays(
    component_file: &str,
    cluster_file: &str,
    temp_dir: &Path,
    overlays: &[PathBuf],
    platform_overlays: &[PathBuf],
) -> Result<()> {
    let mut args = vec![
        "-f".to_string(),
        paths::CLUSTER_SCHEMA.to_string(),
        "-f".to_string(),
        cluster_file.to_string(),
        "-f".to_string(),
        component_file.to_string(),
    ];

    for overlay in platform_overlays {
        args.push("-f".to_string());
        args.push(overlay.to_string_lossy().to_string());
    }

    if !overlays.is_empty() {
        output::detail(&format!("{} pre-render overlay(s)", overlays.len()));
        for overlay in overlays {
            args.push("-f".to_string());
            args.push(overlay.to_string_lossy().to_string());
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
    overlays: &[PathBuf],
    platform_overlays: &[PathBuf],
) -> Result<String> {
    let mut args = vec![
        "--ignore-unknown-comments".to_string(),
        "-f".to_string(),
        paths::CLUSTER_SCHEMA.to_string(),
        "-f".to_string(),
        cluster_file.to_string(),
        "-f".to_string(),
        manifests_path.to_string_lossy().to_string(),
    ];

    for overlay in platform_overlays {
        args.push("-f".to_string());
        args.push(overlay.to_string_lossy().to_string());
    }

    if !overlays.is_empty() {
        output::detail(&format!("{} post-render overlay(s)", overlays.len()));
        for overlay in overlays {
            args.push("-f".to_string());
            args.push(overlay.to_string_lossy().to_string());
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
