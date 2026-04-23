use anyhow::{Result, Context};
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashSet;
use std::process::Command;
use std::sync::Mutex;
use std::path::Path;
use std::io::Write;
use super::discovery::*;
use crate::commands::{resolve_registry_credentials, run_cmd, TaskFailure, report_failures};
use crate::output;

pub fn mirror_chart_images(
    components: &[&ComponentInfo],
    dir: &str,
    registry: Option<String>,
    registry_user: Option<String>,
    registry_pass: Option<String>,
) -> Result<()> {
    output::section("Mirroring container images");

    let creds = resolve_registry_credentials(registry, registry_user, registry_pass)?;

    run_cmd("crane", &["version"])?;

    output::cmd(
        "crane",
        &["auth", "login", &creds.url, "-u", &creds.username, "-p", "***"],
    );
    let login_output = Command::new("crane")
        .args([
            "auth",
            "login",
            &creds.url,
            "-u",
            &creds.username,
            "-p",
            &creds.password,
        ])
        .output()
        .context("Failed to authenticate crane with registry")?;

    if !login_output.status.success() {
        let stderr = String::from_utf8_lossy(&login_output.stderr);
        anyhow::bail!("crane auth login failed: {}", stderr.trim());
    }

    let mut all_images = HashSet::new();

    for component in components {
        let chart_file = format!(
            "{}/{}-{}.tgz",
            dir, component.chart, component.version
        );

        if !Path::new(&chart_file).exists() {
            continue;
        }

        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().to_string_lossy().to_string();

        output::cmd("tar", &["-xzf", &chart_file, "-C", &temp_path]);
        let extract_output = Command::new("tar")
            .args(["-xzf", &chart_file, "-C", &temp_path])
            .output();

        if extract_output.is_err() || !extract_output.as_ref().unwrap().status.success() {
            continue;
        }

        let chart_dir = format!("{}/{}", temp_path, component.chart);

        output::cmd("helm", &["template", "temp-release", &chart_dir]);
        let template_output = Command::new("helm")
            .args(["template", "temp-release", &chart_dir])
            .output();

        if let Ok(result) = template_output {
            if result.status.success() {
                let rendered_yaml = String::from_utf8_lossy(&result.stdout);
                if let Ok(images) = inspect_images(&rendered_yaml) {
                    all_images.extend(images);
                }
            }
        }
    }

    if all_images.is_empty() {
        output::log("No images found to mirror");
        return Ok(());
    }

    let images_vec: Vec<_> = all_images.into_iter().collect();
    let pb = output::progress_bar(images_vec.len() as u64);
    let failures = Mutex::new(Vec::<TaskFailure>::new());

    images_vec.par_iter().for_each(|image| {
        pb.set_message(image.clone());

        if let Err(e) =
            mirror_single_image(image, &creds.url, &creds.username, &creds.password)
        {
            failures.lock().unwrap().push(TaskFailure::new(
                image.clone(),
                format!("{}", e),
            ));
        }

        pb.inc(1);
    });

    pb.finish_and_clear();

    let failures = failures.into_inner().unwrap();
    if !failures.is_empty() {
        report_failures(&failures);
        anyhow::bail!("{} image mirrors failed", failures.len());
    }

    Ok(())
}

fn inspect_images(manifests: &str) -> Result<HashSet<String>> {
    output::cmd("kbld", &["inspect", "-f", "-", "--json"]);
    let mut child = Command::new("kbld")
        .args(["inspect", "-f", "-", "--json"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start kbld inspect")?;

    child
        .stdin
        .take()
        .unwrap()
        .write_all(manifests.as_bytes())?;

    let output = child
        .wait_with_output()
        .context("Failed to run kbld inspect")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("kbld inspect failed: {}", stderr);
    }

    let parsed: Value =
        serde_json::from_slice(&output.stdout).context("Failed to parse kbld inspect output")?;

    let mut images = HashSet::new();
    if let Some(tables) = parsed["Tables"].as_array() {
        for table in tables {
            if let Some(rows) = table["Rows"].as_array() {
                for row in rows {
                    if let Some(image) = row["image"].as_str() {
                        images.insert(image.to_string());
                    }
                }
            }
        }
    }

    Ok(images)
}

fn strip_digest(image: &str) -> &str {
    if image.contains(':') && image.contains('@') {
        &image[..image.find('@').unwrap()]
    } else {
        image
    }
}

fn mirror_single_image(
    source_image: &str,
    registry_url: &str,
    _username: &str,
    _password: &str,
) -> Result<()> {
    let clean_image = strip_digest(source_image);
    let target_image = replace_registry(clean_image, registry_url);
    let source_ref = normalize_image_ref(source_image);

    run_cmd("crane", &["copy", &source_ref, &target_image])?;
    Ok(())
}

fn normalize_image_ref(source_image: &str) -> String {
    let image = strip_digest(source_image);

    let has_registry = image
        .split('/')
        .next()
        .is_some_and(|first| first.contains('.'));

    if has_registry {
        image.to_string()
    } else if image.contains('/') {
        format!("docker.io/{}", image)
    } else {
        format!("docker.io/library/{}", image)
    }
}

fn replace_registry(source_image: &str, target_registry: &str) -> String {
    let known_registries = [
        "docker.io/",
        "gcr.io/",
        "ghcr.io/",
        "quay.io/",
        "registry.k8s.io/",
        "k8s.gcr.io/",
    ];

    for registry in &known_registries {
        if let Some(image_path) = source_image.strip_prefix(registry) {
            return format!("{}/{}", target_registry, image_path);
        }
    }

    if let Some(first_slash) = source_image.find('/') {
        let potential_registry = &source_image[..first_slash];
        if potential_registry.contains('.') || potential_registry.contains(':') {
            let image_path = &source_image[first_slash + 1..];
            return format!("{}/{}", target_registry, image_path);
        }
    }

    if !source_image.contains('/') {
        format!("{}/library/{}", target_registry, source_image)
    } else {
        format!("{}/{}", target_registry, source_image)
    }
}
