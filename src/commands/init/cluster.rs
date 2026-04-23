use anyhow::{Context, Result, bail};
use dialoguer::{Input, Select, theme::ColorfulTheme};
use std::fs;
use std::path::Path;
use crate::output;

/// Create a new cluster configuration interactively
pub fn init_cluster() -> Result<()> {
    output::header("Init Cluster");

    let cluster_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Cluster name")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.is_empty() {
                Err("Cluster name cannot be empty")
            } else if !input.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                Err("Cluster name must contain only alphanumeric characters, hyphens, and underscores")
            } else {
                Ok(())
            }
        })
        .interact_text()
        .context("Failed to get cluster name")?;

    let cloud_options = vec!["aws", "azure", "onprem"];
    let cloud_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Cloud provider")
        .items(&cloud_options)
        .default(0)
        .interact()
        .context("Failed to get cloud provider")?;
    let cloud = cloud_options[cloud_selection];

    let region: String = if cloud != "onprem" {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Region")
            .default(if cloud == "aws" {
                "us-east-1".into()
            } else {
                "eastus".into()
            })
            .interact_text()
            .context("Failed to get region")?
    } else {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Datacenter")
            .default("dc1".into())
            .interact_text()
            .context("Failed to get datacenter")?
    };

    let env_options = vec!["dev", "staging", "prod"];
    let env_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Environment")
        .items(&env_options)
        .default(0)
        .interact()
        .context("Failed to get environment")?;
    let environment = env_options[env_selection];

    let aws_config = if cloud == "aws" {
        let account_id: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("AWS Account ID")
            .validate_with(|input: &String| -> Result<(), &str> {
                if input.len() == 12 && input.chars().all(|c| c.is_numeric()) {
                    Ok(())
                } else {
                    Err("AWS Account ID must be 12 digits")
                }
            })
            .interact_text()
            .context("Failed to get AWS account ID")?;

        Some(account_id)
    } else {
        None
    };

    let cluster_dir = Path::new("platform/clusters").join(&cluster_name);

    if cluster_dir.exists() {
        bail!("Cluster directory '{}' already exists", cluster_dir.display());
    }

    fs::create_dir_all(&cluster_dir)
        .with_context(|| format!("Failed to create cluster directory: {}", cluster_dir.display()))?;

    fs::create_dir_all(cluster_dir.join("overlays"))
        .context("Failed to create overlays directory")?;

    let mut cluster_yaml = format!(r#"#@data/values
---
#! Physical facts
cluster_name: "{}"
cloud: {}
region: {}
environment: {}
overlays:
  - {}
  - {}
"#, cluster_name, cloud, region, environment, cloud, environment);

    if let Some(account_id) = aws_config {
        cluster_yaml.push_str(&format!(r#"
#! AWS-specific configuration
aws:
  account_id: "{}"
  #! Note: Tenant permission boundary is auto-provisioned by platform/components/cloud-permissions
  #! Policy ARN: arn:aws:iam::{{account_id}}:policy/{{cluster_name}}-TenantMaxPermissions
"#, account_id));
    }

    cluster_yaml.push_str(r#"
#! Tenants (Application teams/projects on this cluster)
#! Add your tenant configurations here
tenants: []
  #! Example:
  #! - name: example-tenant
  #!   owners:
  #!     - name: "team-leads"
  #!       kind: Group
"#);

    fs::write(cluster_dir.join("cluster.yaml"), cluster_yaml)
        .context("Failed to create cluster.yaml")?;

    output::done(&format!("Cluster '{}' created", cluster_name));
    output::summary(&format!("{}", cluster_dir.display()));
    output::detail("edit cluster.yaml to customize, then: fedcore bootstrap --cluster <path>");

    Ok(())
}
