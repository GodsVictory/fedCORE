use anyhow::{Context, Result, bail};
use dialoguer::{Input, theme::ColorfulTheme};
use std::fs;
use std::path::Path;
use crate::output;
use super::templates::*;

/// Initialize a new FedCore project directory structure
pub fn init_project() -> Result<()> {
    output::header("Init Project");

    let project_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Project name")
        .default("my-fedcore-project".into())
        .interact_text()
        .context("Failed to get project name")?;

    let project_path = Path::new(&project_name);

    if project_path.exists() {
        bail!("Directory '{}' already exists", project_name);
    }

    let dirs = vec![
        project_path.to_path_buf(),
        project_path.join("platform"),
        project_path.join("platform/clusters"),
        project_path.join("platform/components"),
        project_path.join("platform/components/tenant-instances"),
        project_path.join("platform/components/tenant-instances/base"),
        project_path.join("platform/components/capsule"),
        project_path.join("platform/components/capsule/base"),
        project_path.join("platform/components/kro"),
        project_path.join("platform/components/kro/base"),
        project_path.join("platform/bootstrap"),
        project_path.join("platform/bootstrap/component-sources"),
        project_path.join("platform/bootstrap/component-sources/base"),
        project_path.join("platform/rgds"),
        project_path.join("platform/rgds/namespace"),
        project_path.join("platform/rgds/namespace/base"),
        project_path.join("docs"),
    ];

    for dir in &dirs {
        fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
        output::detail(&format!("mkdir {}", dir.display()));
    }

    create_main_readme(project_path, &project_name)?;
    create_platform_readmes(project_path)?;
    create_documentation_files(project_path)?;
    create_bootstrap_files(project_path)?;
    create_default_components(project_path)?;
    create_example_cluster(project_path)?;

    output::done("Project initialized");
    output::summary(&format!("cd {} && fedcore init cluster", project_name));

    Ok(())
}

fn create_main_readme(project_path: &Path, project_name: &str) -> Result<()> {
    let readme_content = format!(r#"# {}

A FedCore platform deployment project.

## Overview

This project contains the configuration and infrastructure definitions for deploying and managing Kubernetes clusters using the FedCore platform. FedCore provides a GitOps-based, multi-cloud platform with multi-tenancy support.

## Project Structure

```
{}
├── docs/                    # Documentation
├── platform/
│   ├── bootstrap/           # Bootstrap templates for cluster initialization
│   ├── clusters/            # Cluster configurations
│   ├── components/          # Component definitions (infrastructure services)
│   └── rgds/               # Resource Group Definitions
└── README.md
```

## Quick Start

1. **Create a cluster configuration:**
   ```bash
   fedcore init cluster
   ```

2. **Create components as needed:**
   ```bash
   fedcore init component
   ```

3. **Build artifacts for your cluster:**
   ```bash
   fedcore build --cluster platform/clusters/<cluster-name> --all
   ```

4. **Bootstrap the cluster:**
   ```bash
   fedcore bootstrap --cluster platform/clusters/<cluster-name> --deploy
   ```

## Documentation

- [Getting Started](docs/README.md) - Introduction and setup guide
- [Project Structure](docs/PROJECT_STRUCTURE.md) - Detailed directory layout
- [CLI Commands](docs/COMMANDS.md) - FedCore CLI reference
- [Development Workflow](docs/WORKFLOW.md) - Day-to-day operations

## Key Concepts

- **Clusters**: Kubernetes clusters with environment-specific configurations
- **Components**: Infrastructure services (Istio, Capsule, monitoring, etc.)
- **Bootstrap**: Initial cluster setup and GitOps configuration
- **OCI Artifacts**: Versioned, immutable component packages
- **GitOps**: Flux-based declarative deployment

## Support

For issues, questions, or contributions, refer to the documentation in the `docs/` directory.
"#, project_name, project_name);

    fs::write(project_path.join("README.md"), readme_content)
        .context("Failed to create README.md")?;
    output::detail("created README.md");
    Ok(())
}

fn create_platform_readmes(project_path: &Path) -> Result<()> {
    let clusters_readme = r#"# Clusters

This directory contains cluster configuration files.

## Creating a New Cluster

Use the CLI to create a new cluster configuration:

```bash
fedcore init cluster
```

## Cluster Configuration

Each cluster should have:
- `cluster.yaml` - Main configuration file
- `overlays/` - Optional overlay files for customization (optional)

See the schema at `platform/clusters/schema.yaml` for all available options.
"#;

    fs::write(project_path.join("platform/clusters/README.md"), clusters_readme)
        .context("Failed to create clusters README.md")?;

    fs::write(project_path.join("platform/clusters/schema.yaml"), CLUSTER_SCHEMA)
        .context("Failed to create clusters schema.yaml")?;

    let components_readme = r#"# Components

This directory contains component definitions for the platform.

## Creating a New Component

Use the CLI to create a new component:

```bash
fedcore init component
```

## Component Types

- **helm** - Helm chart based components
- **manifests** - Plain Kubernetes manifest components
- **kustomize** - Kustomize based components

Each component should have a `component.yaml` file describing its metadata and configuration.
"#;

    fs::write(project_path.join("platform/components/README.md"), components_readme)
        .context("Failed to create components README.md")?;

    Ok(())
}

fn create_documentation_files(project_path: &Path) -> Result<()> {
    let docs_dir = project_path.join("docs");

    fs::write(docs_dir.join("README.md"), "# FedCore Platform Documentation\n")
        .context("Failed to create docs/README.md")?;
    fs::write(docs_dir.join("PROJECT_STRUCTURE.md"), "# Project Structure\n")
        .context("Failed to create docs/PROJECT_STRUCTURE.md")?;
    fs::write(docs_dir.join("COMMANDS.md"), "# FedCore CLI Commands\n")
        .context("Failed to create docs/COMMANDS.md")?;
    fs::write(docs_dir.join("WORKFLOW.md"), "# Development Workflow\n")
        .context("Failed to create docs/WORKFLOW.md")?;

    Ok(())
}

fn create_bootstrap_files(project_path: &Path) -> Result<()> {
    let bootstrap_base = project_path.join("platform/bootstrap/component-sources/base");

    fs::write(bootstrap_base.join("README.md"), BOOTSTRAP_README)
        .context("Failed to create bootstrap README.md")?;
    fs::write(bootstrap_base.join("component-sources.yaml"), BOOTSTRAP_COMPONENT_SOURCES)
        .context("Failed to create component-sources.yaml")?;
    fs::write(bootstrap_base.join("flux-ca-certificates.yaml"), BOOTSTRAP_FLUX_CA_CERTS)
        .context("Failed to create flux-ca-certificates.yaml")?;

    Ok(())
}

fn create_default_components(project_path: &Path) -> Result<()> {
    let tenant_instances_dir = project_path.join("platform/components/tenant-instances");
    fs::write(tenant_instances_dir.join("README.md"), TENANT_INSTANCES_README)?;
    fs::write(tenant_instances_dir.join("base/tenant-instances.yaml"), TENANT_INSTANCES_YAML)?;

    let namespace_rgd_dir = project_path.join("platform/rgds/namespace");
    fs::write(namespace_rgd_dir.join("README.md"), NAMESPACE_RGD_README)?;
    fs::write(namespace_rgd_dir.join("base/namespace-rgd.yaml"), NAMESPACE_RGD_YAML)?;

    let capsule_dir = project_path.join("platform/components/capsule");
    fs::write(capsule_dir.join("README.md"), CAPSULE_README)?;
    fs::write(capsule_dir.join("component.yaml"), CAPSULE_COMPONENT_YAML)?;
    fs::write(capsule_dir.join("default-values.yaml"), CAPSULE_DEFAULT_VALUES)?;
    fs::write(capsule_dir.join("base/namespace.yaml"), CAPSULE_NAMESPACE_YAML)?;

    let kro_dir = project_path.join("platform/components/kro");
    fs::write(kro_dir.join("README.md"), KRO_README)?;
    fs::write(kro_dir.join("base/install.yaml"), KRO_INSTALL_YAML)?;
    fs::write(kro_dir.join("base/core-resources-rbac.yaml"), KRO_CORE_RBAC)?;
    fs::write(kro_dir.join("base/platform-fedcore-rbac.yaml"), KRO_PLATFORM_RBAC)?;
    fs::write(kro_dir.join("base/default-clusterroles-rbac.yaml"), KRO_DEFAULT_ROLES_RBAC)?;
    fs::write(kro_dir.join("base/enable-crd-deletion.yaml"), KRO_ENABLE_CRD_DELETION)?;
    fs::write(kro_dir.join("base/image-overlay.yaml"), KRO_IMAGE_OVERLAY)?;

    Ok(())
}

fn create_example_cluster(project_path: &Path) -> Result<()> {
    let example_cluster_dir = project_path.join("platform/clusters/example-cluster");
    fs::create_dir_all(&example_cluster_dir)?;
    fs::create_dir_all(example_cluster_dir.join("overlays"))?;

    let cluster_yaml = r#"#@data/values
---
#! Physical facts
cluster_name: "example-cluster"
cloud: aws
region: us-east-1
environment: dev
overlays:
  - aws
  - dev

#! AWS-specific configuration
aws:
  account_id: "123456789012"

#! Example tenants
tenants:
  - name: acme
    type: namespace
    description: "Acme Corp development namespace"
    team: "platform"
    cost_center: "engineering"
    owners:
      - kind: Group
        name: acme-admins
    role_bindings:
      - kind: Group
        name: acme-admins
        role: admin
      - kind: Group
        name: acme-developers
        role: edit

#! Components deployed to this cluster
components:
  - name: capsule
    enabled: true
    version: "latest"
  - name: kro
    enabled: true
    version: "latest"
  - name: namespace
    enabled: true
    version: "latest"
    depends_on: [kro]
  - name: tenant-instances
    enabled: true
    version: "latest"
    depends_on: [namespace]
"#;

    fs::write(example_cluster_dir.join("cluster.yaml"), cluster_yaml)?;

    let overlay_yaml = r#"#@ load("@ytt:overlay", "overlay")

#! Example Cluster Overlay
#@overlay/match by=overlay.subset({"kind": "Deployment"}), expects="0+"
---
spec:
  template:
    spec:
      containers:
      #@overlay/match by=overlay.all, expects="1+"
      - resources:
          #@overlay/match missing_ok=True
          limits:
            #@overlay/match missing_ok=True
            cpu: "1000m"
            #@overlay/match missing_ok=True
            memory: "1Gi"
          #@overlay/match missing_ok=True
          requests:
            #@overlay/match missing_ok=True
            cpu: "100m"
            #@overlay/match missing_ok=True
            memory: "128Mi"
"#;

    fs::write(example_cluster_dir.join("overlays/resource-limits.yaml"), overlay_yaml)?;

    Ok(())
}
