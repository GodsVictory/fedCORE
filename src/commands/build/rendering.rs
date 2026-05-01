use anyhow::Result;
use std::path::Path;
use std::fs;
use crate::commands::run_cmd;
use crate::helm;
use crate::output;
use crate::types::HelmConfig;

pub fn render_helm_chart(
    temp_dir: &Path,
    helm: &HelmConfig,
    release_name: &str,
    namespace: &str,
) -> Result<()> {
    output::detail(&format!("helm template {}:{}", helm.chart, helm.version));

    let values_file = temp_dir.join("values.yaml");
    fs::write(&values_file, serde_saphyr::to_string(&helm.values)?)?;

    let repo = if helm.resolved_chart_ref.is_empty() {
        helm.source_repo.clone()
    } else {
        helm.resolved_chart_ref.clone()
    };
    let chart_ref = format!("{}/{}", repo, helm.chart);
    let chart_path = helm::resolve_cached_chart(&helm.chart, &helm.version, &chart_ref)?;
    output::detail(&format!("using chart {}", chart_path));

    let mut helm_args = vec![
        "template".to_string(),
        release_name.to_string(),
        chart_path,
        "--namespace".to_string(),
        namespace.to_string(),
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
