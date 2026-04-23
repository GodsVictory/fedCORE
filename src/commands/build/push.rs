use anyhow::{Result, bail};
use std::fs;
use crate::output;
use crate::types::BuildMatrixEntry;

pub fn push_artifacts(
    matrix: &[BuildMatrixEntry],
    registry: &str,
    version: &str,
    repo_url: &str,
    ref_name: &str,
    sha: &str,
) -> Result<()> {
    let username = std::env::var("OCI_REGISTRY_USER")?;
    let password = std::env::var("OCI_REGISTRY_PASS")?;
    let artifact_count = matrix.len();

    output::section(&format!("Pushing {} artifacts", artifact_count));

    let mut failed_pushes = Vec::new();

    for (i, artifact) in matrix.iter().enumerate() {
        output::progress(i + 1, artifact_count, &artifact.target_name);

        let oci_layout_dir = format!("oci-layout/{}", artifact.target_name);
        fs::create_dir_all(&oci_layout_dir)?;
        fs::copy(
            format!("dist/{}.yaml", artifact.target_name),
            format!("{}/platform.yaml", oci_layout_dir),
        )?;

        let oci_ref = format!(
            "oci://{}/fedcore/{}:{}",
            registry, artifact.target_name, version
        );
        let creds = format!("{}:{}", username, password);

        let path_arg = format!("--path={}", oci_layout_dir);
        let source_arg = format!("--source={}", repo_url);
        let revision_arg = format!("--revision={}@sha1:{}", ref_name, sha);
        let creds_arg = format!("--creds={}", creds);

        output::cmd(
            "flux",
            &[
                "push",
                "artifact",
                &oci_ref,
                &path_arg,
                &source_arg,
                &revision_arg,
                "--creds=***",
            ],
        );
        let result = std::process::Command::new("flux")
            .args([
                "push",
                "artifact",
                &oci_ref,
                &path_arg,
                &source_arg,
                &revision_arg,
                &creds_arg,
            ])
            .output()?;

        if result.status.success() {
            output::progress_done(true);
        } else {
            output::progress_done(false);
            failed_pushes.push(artifact.target_name.clone());
        }
    }

    if failed_pushes.is_empty() {
        output::done(&format!("Pushed {} artifacts", artifact_count));
    } else {
        for name in &failed_pushes {
            output::item_fail(name);
        }
        output::fail(&format!(
            "{}/{} failed to push",
            failed_pushes.len(),
            artifact_count
        ));
        bail!("Push failed");
    }

    Ok(())
}
