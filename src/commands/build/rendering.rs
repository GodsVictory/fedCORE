use anyhow::{Result, Context};
use std::path::Path;
use std::fs;
use crate::commands::run_cmd;
use crate::helm;
use crate::output;
use crate::paths;
use crate::types::MergedComponent;

pub fn render_helm_chart(temp_dir: &Path) -> Result<()> {
    let component_file = temp_dir.join("component-merged.yaml");
    let component_data: MergedComponent =
        serde_yaml::from_str(&fs::read_to_string(&component_file)?)?;

    let helm = component_data
        .helm
        .context("helm section missing in component.yaml")?;

    output::detail(&format!("helm template {}:{}", helm.chart, helm.version));

    let values_file = temp_dir.join("values.yaml");
    fs::write(&values_file, serde_yaml::to_string(&helm.values)?)?;

    let chart_ref = format!("{}/{}", helm.mirror_repo, helm.chart);
    let chart_path = helm::resolve_cached_chart(&helm.chart, &helm.version, &chart_ref)?;
    output::detail(&format!("using chart {}", chart_path));

    let mut helm_args = vec![
        "template".to_string(),
        helm.release.name,
        chart_path,
        "--namespace".to_string(),
        helm.release.namespace,
        "--values".to_string(),
        values_file.to_string_lossy().to_string(),
    ];
    for flag in &helm.flags {
        helm_args.push(flag.clone());
    }

    let helm_args_str: Vec<&str> = helm_args.iter().map(|s| s.as_str()).collect();
    let stdout = run_cmd("helm", &helm_args_str)?;

    fs::write(temp_dir.join("helm-rendered.yaml"), stdout)?;
    Ok(())
}

pub fn render_base_manifests(
    cluster_file: &str,
    base_dir: &str,
    manifests_path: &Path,
) -> Result<()> {
    let manifests_str = manifests_path.to_string_lossy();
    let stdout = run_cmd(
        "ytt",
        &[
            "-f",
            paths::CLUSTER_SCHEMA,
            "-f",
            cluster_file,
            "-f",
            &manifests_str,
            "-f",
            base_dir,
        ],
    )?;
    fs::write(manifests_path, stdout)?;
    Ok(())
}
