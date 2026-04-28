use anyhow::Result;
use std::path::Path;
use std::fs;
use serde_json::Value;
use crate::commands::run_cmd;
use crate::helm;
use crate::output;
use crate::paths;
use crate::types::HelmConfig;

pub fn apply_component_overrides(
    merged_path: &Path,
    component_data_values_yaml: &str,
) -> Result<()> {
    let data: Value = serde_saphyr::from_str(component_data_values_yaml).unwrap_or_default();
    let component = data.get("component").and_then(|c| c.as_object());

    if let Some(overrides) = component {
        if !overrides.is_empty() {
            let mut merged: Value = serde_saphyr::from_str(&fs::read_to_string(merged_path)?)?;
            deep_merge(&mut merged, &Value::Object(overrides.clone()));
            fs::write(merged_path, serde_saphyr::to_string(&merged)?)?;
        }
    }

    Ok(())
}

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

pub fn render_base_manifests(
    cluster_file: &str,
    base_dir: &str,
    manifests_path: &Path,
    component_data_values_yaml: &str,
) -> Result<()> {
    let manifests_str = manifests_path.to_string_lossy();
    let data_values_file = manifests_path.with_extension("component-values.yaml");
    fs::write(&data_values_file, component_data_values_yaml)?;
    let data_values_str = data_values_file.to_string_lossy();
    let stdout = run_cmd(
        "ytt",
        &[
            "-f", paths::CLUSTER_SCHEMA,
            "-f", cluster_file,
            "-f", &manifests_str,
            "-f", base_dir,
            "--data-values-file", &data_values_str,
        ],
    )?;
    fs::write(manifests_path, stdout)?;
    Ok(())
}

fn deep_merge(base: &mut Value, overrides: &Value) {
    match (base, overrides) {
        (Value::Object(base_map), Value::Object(override_map)) => {
            for (key, override_val) in override_map {
                let entry = base_map.entry(key.clone()).or_insert(Value::Null);
                deep_merge(entry, override_val);
            }
        }
        (base, overrides) => {
            *base = overrides.clone();
        }
    }
}
