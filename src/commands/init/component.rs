use anyhow::{Context, Result, bail};
use dialoguer::{Input, Select, theme::ColorfulTheme};
use std::fs;
use std::path::Path;
use crate::output;

/// Create a new component interactively
pub fn init_component() -> Result<()> {
    output::header("Init Component");

    let component_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Component name")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.is_empty() {
                Err("Component name cannot be empty")
            } else if !input.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                Err("Component name must contain only alphanumeric characters, hyphens, and underscores")
            } else {
                Ok(())
            }
        })
        .interact_text()
        .context("Failed to get component name")?;

    let type_options = vec!["helm", "manifests", "kustomize"];
    let type_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Component type")
        .items(&type_options)
        .default(0)
        .interact()
        .context("Failed to get component type")?;
    let component_type = type_options[type_selection];

    let component_dir = Path::new("platform/components").join(&component_name);

    if component_dir.exists() {
        bail!("Component directory '{}' already exists", component_dir.display());
    }

    fs::create_dir_all(&component_dir)
        .with_context(|| format!("Failed to create component directory: {}", component_dir.display()))?;

    fs::create_dir_all(component_dir.join("base"))
        .context("Failed to create base directory")?;

    fs::create_dir_all(component_dir.join("overlays"))
        .context("Failed to create overlays directory")?;

    let component_yaml = match component_type {
        "helm" => create_helm_component(&component_name)?,
        "manifests" => create_manifests_component(&component_name)?,
        "kustomize" => create_kustomize_component(&component_name)?,
        _ => unreachable!(),
    };

    fs::write(component_dir.join("component.yaml"), component_yaml)
        .context("Failed to create component.yaml")?;

    let namespace_yaml = format!(r#"apiVersion: v1
kind: Namespace
metadata:
  name: {}
  labels:
    name: {}
"#, component_name, component_name);

    fs::write(component_dir.join("base/namespace.yaml"), namespace_yaml)
        .context("Failed to create namespace.yaml")?;

    if component_type == "helm" {
        let default_values = format!(r#"#@data/values
---
#! Default values for {} component
#! Override these in cluster-specific overlays

helm_repositories:
  use_mirror: false
  oci_registry_url: "oci://registry.example.com/helm-charts"
"#, component_name);

        fs::write(component_dir.join("default-values.yaml"), default_values)
            .context("Failed to create default-values.yaml")?;
    }

    let readme_content = format!(r#"# {}

Component type: {}

## Description

Add a description of this component here.

## Configuration

Edit `component.yaml` to configure this component.

## Overlays

Add overlays in the `overlays/` directory, named by overlay ID.
Each cluster's `overlays` list (e.g., `[aws, prod]`) selects which
directories are applied during build.

Example structure:
```
overlays/
  aws/
    overlay.yaml
  azure/
    overlay.yaml
  prod/
    overlay.yaml
```
"#, component_name, component_type);

    fs::write(component_dir.join("README.md"), readme_content)
        .context("Failed to create README.md")?;

    output::done(&format!("Component '{}' created", component_name));
    output::summary(&format!("{}", component_dir.display()));
    output::detail("edit component.yaml to customize, add manifests to base/");

    Ok(())
}

fn create_helm_component(component_name: &str) -> Result<String> {
    let chart_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Helm chart name")
        .default(component_name.to_string())
        .interact_text()
        .context("Failed to get chart name")?;

    let chart_repo: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Helm chart repository URL")
        .default("https://charts.example.com".into())
        .interact_text()
        .context("Failed to get chart repository")?;

    let chart_version: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Helm chart version")
        .default("1.0.0".into())
        .interact_text()
        .context("Failed to get chart version")?;

    let namespace: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Kubernetes namespace")
        .default(component_name.to_string())
        .interact_text()
        .context("Failed to get namespace")?;

    Ok(format!(r#"#@ load("@ytt:data", "data")
---
#! Helm chart configuration
helm:
  sourceRepo: {}
  chart: {}
  version: "{}"
  mirrorRepo: #@ data.values.helm_repositories.oci_registry_url if data.values.helm_repositories.use_mirror else "{}"

  #! Release configuration
  release:
    name: {}
    namespace: {}

  #! Values for helm template
  values:
    replicaCount: 2

    resources:
      requests:
        cpu: 100m
        memory: 128Mi
      limits:
        cpu: 500m
        memory: 512Mi
"#, chart_repo, chart_name, chart_version, chart_repo, component_name, namespace))
}

fn create_manifests_component(component_name: &str) -> Result<String> {
    let namespace: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Kubernetes namespace")
        .default(component_name.to_string())
        .interact_text()
        .context("Failed to get namespace")?;

    Ok(format!(r#"#@ load("@ytt:data", "data")
---
#! Namespace configuration
namespace: {}

#! Component description
description: "Custom Kubernetes manifests for {}"
"#, namespace, component_name))
}

fn create_kustomize_component(component_name: &str) -> Result<String> {
    let namespace: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Kubernetes namespace")
        .default(component_name.to_string())
        .interact_text()
        .context("Failed to get namespace")?;

    Ok(format!(r#"#@ load("@ytt:data", "data")
---
#! Namespace configuration
namespace: {}

#! Kustomize configuration
kustomize:
  path: base
"#, namespace))
}
