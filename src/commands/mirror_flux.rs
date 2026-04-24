use anyhow::{Result, Context, bail};
use regex::Regex;
use crate::commands::run_cmd;
use crate::output;

pub fn execute(registry: Option<&str>) -> Result<()> {
    output::header("Mirror Flux");

    let target_registry = match registry {
        Some(r) => r.to_string(),
        None => std::env::var("OCI_REGISTRY")
            .context("--registry or OCI_REGISTRY required")?,
    };

    output::summary(&format!("target: {}", target_registry));

    output::section("Extracting images");

    let stdout = run_cmd(
        "flux",
        &[
            "install",
            "--components-extra=image-reflector-controller,image-automation-controller",
            "--export",
        ],
    )?;

    let manifest = String::from_utf8_lossy(&stdout);
    let image_regex = Regex::new(r"image: (.+)")?;
    let mut images: Vec<String> = Vec::new();

    for line in manifest.lines() {
        if let Some(captures) = image_regex.captures(line) {
            if let Some(image) = captures.get(1) {
                let image_str = image.as_str().trim().to_string();
                if !images.contains(&image_str) {
                    images.push(image_str);
                }
            }
        }
    }

    if images.is_empty() {
        bail!("No images found in manifest");
    }

    for image in &images {
        output::item_ok(image);
    }

    output::section(&format!("Mirroring {} images", images.len()));

    for (i, source_image) in images.iter().enumerate() {
        let image_path = source_image.replace("ghcr.io/", "");
        let target_image = format!("{}/{}", target_registry, image_path);

        output::progress(i + 1, images.len(), source_image);
        output::detail(&format!("-> {}", target_image));

        match run_cmd(
            "crane",
            &["copy", "--platform=all", source_image, &target_image],
        ) {
            Ok(_) => output::progress_done(true),
            Err(e) => {
                output::progress_done(false);
                bail!("Failed to mirror image: {}", e);
            }
        }
    }

    output::done("All Flux images mirrored");
    output::log("To install in airgapped environment:");
    output::log(&format!(
        "  flux install --registry={}/fluxcd \\",
        target_registry
    ));
    output::log(
        "    --components-extra=image-reflector-controller,image-automation-controller",
    );

    Ok(())
}
