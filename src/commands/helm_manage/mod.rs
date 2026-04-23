mod discovery;
mod download;
mod push;
mod mirror;

use anyhow::{Result, bail};
use crate::output;
use std::path::Path;
pub use discovery::*;
pub use download::*;
pub use push::*;
pub use mirror::*;

#[derive(Debug, clap::Args)]
pub struct HelmManageArgs {
    /// Download specific artifact(s) (comma-separated)
    #[arg(short, long, default_value = "all")]
    pub artifact: String,

    /// Output directory for charts
    #[arg(short, long, default_value = "./helm-charts")]
    pub dir: String,

    /// Discover latest versions from upstream
    #[arg(short, long)]
    pub latest: bool,

    /// Update component YAML files (requires --latest)
    #[arg(short, long)]
    pub update: bool,

    /// Push charts to OCI registry
    #[arg(short, long)]
    pub push: bool,

    /// Extract, pull, and push container images from charts
    #[arg(short, long)]
    pub mirror_images: bool,

    /// OCI registry URL
    #[arg(short, long)]
    pub registry: Option<String>,

    /// OCI registry username
    #[arg(long)]
    pub registry_user: Option<String>,

    /// OCI registry password
    #[arg(long)]
    pub registry_pass: Option<String>,
}

pub fn execute(args: HelmManageArgs) -> Result<()> {
    output::header("Helm Manage");

    output::config("artifacts", &args.artifact);
    output::config("output", &args.dir);
    output::config("latest", &args.latest.to_string());
    output::config("push", &args.push.to_string());
    output::config("mirror", &args.mirror_images.to_string());

    if args.push || args.mirror_images {
        let has_registry = args.registry.is_some() || std::env::var("OCI_REGISTRY").is_ok();
        let has_user = args.registry_user.is_some() || std::env::var("OCI_REGISTRY_USER").is_ok();
        let has_pass = args.registry_pass.is_some() || std::env::var("OCI_REGISTRY_PASS").is_ok();

        if !has_registry || !has_user || !has_pass {
            output::warn("Missing registry credentials (--registry/--registry-user/--registry-pass or env vars)");
        }
    }

    let components = discover_components()?;
    output::log(&format!("Discovered {} components", components.len()));

    std::fs::create_dir_all(&args.dir)?;

    let filtered_components: Vec<&ComponentInfo> = if args.artifact == "all" {
        components.iter().collect()
    } else {
        let artifact_list: Vec<&str> = args.artifact.split(',').map(|s| s.trim()).collect();
        components.iter()
            .filter(|c| {
                artifact_list.iter().any(|a| {
                    let normalized = a.trim_end_matches('/');
                    let basename = Path::new(normalized).file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default();
                    c.name == basename || c.name == normalized
                })
            })
            .collect()
    };

    if filtered_components.is_empty() {
        bail!("No components to process");
    }

    if args.latest {
        discover_latest_versions(&filtered_components, &args.dir, args.update)?;
    } else {
        download_current_versions(&filtered_components, &args.dir)?;
    }

    if args.push {
        push_charts(&filtered_components, &args.dir, args.registry.clone(), args.registry_user.clone(), args.registry_pass.clone())?;
    }

    if args.mirror_images {
        mirror_chart_images(&filtered_components, &args.dir, args.registry, args.registry_user, args.registry_pass)?;
    }

    output::done("Helm charts managed successfully");

    Ok(())
}
