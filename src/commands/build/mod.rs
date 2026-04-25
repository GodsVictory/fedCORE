mod utils;
mod overlays;
mod rendering;
mod artifacts;
mod push;

use anyhow::{Result, bail};

pub(crate) use artifacts::build_single_artifact;
pub use artifacts::build_artifacts;
pub use push::push_artifacts;

pub fn validate_build(entry: &crate::types::BuildMatrixEntry) -> Result<bool> {
    match build_single_artifact(entry, false) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[derive(Debug, clap::Args)]
pub struct BuildArgs {
    /// Artifact path (e.g., platform/components/capsule)
    #[arg(short, long)]
    pub artifact: Option<String>,

    /// Cluster directory (e.g., platform/clusters/mycluster)
    #[arg(short, long)]
    pub cluster: Option<String>,

    /// Component instance ID (filters to a specific instance when a component has multiple)
    #[arg(short, long)]
    pub id: Option<String>,

    /// Build all artifacts for all clusters
    #[arg(long)]
    pub all: bool,

    /// Push artifacts to OCI registry after building
    #[arg(short, long)]
    pub push: bool,

    /// OCI registry URL
    #[arg(short, long)]
    pub registry: Option<String>,

    /// Artifact version tag
    #[arg(short, long)]
    pub version: Option<String>,

    /// Git repository URL
    #[arg(long)]
    pub repo_url: Option<String>,

    /// Git ref name (branch/tag)
    #[arg(long)]
    pub ref_name: Option<String>,

    /// Git commit SHA
    #[arg(long)]
    pub sha: Option<String>,
}

struct PushConfig {
    registry: String,
    version: String,
    repo_url: String,
    ref_name: String,
    sha: String,
}

fn resolve_push_config(args: &BuildArgs) -> Result<PushConfig> {
    use crate::commands::run_command_stdout;
    use crate::output;

    let registry = args.registry.clone()
        .or_else(|| std::env::var("OCI_REGISTRY").ok())
        .ok_or_else(|| anyhow::anyhow!("Push requires --registry or OCI_REGISTRY"))?;
    let version = args.version.clone().unwrap_or_else(|| "latest".to_string());
    let repo_url = args.repo_url.clone()
        .or_else(|| run_command_stdout("git", &["config", "--get", "remote.origin.url"]))
        .ok_or_else(|| anyhow::anyhow!("Push requires --repo-url or git remote origin"))?;
    let ref_name = args.ref_name.clone()
        .or_else(|| run_command_stdout("git", &["rev-parse", "--abbrev-ref", "HEAD"]))
        .ok_or_else(|| anyhow::anyhow!("Push requires --ref-name or a git branch"))?;
    let sha = args.sha.clone()
        .or_else(|| run_command_stdout("git", &["rev-parse", "HEAD"]))
        .ok_or_else(|| anyhow::anyhow!("Push requires --sha or a git repo"))?;

    if std::env::var("OCI_REGISTRY_USER").is_err()
        || std::env::var("OCI_REGISTRY_PASS").is_err()
    {
        bail!("Push requires OCI_REGISTRY_USER and OCI_REGISTRY_PASS env vars");
    }

    output::config("registry", &registry);
    output::config("version", &version);
    output::config("repo", &repo_url);
    output::config("ref", &ref_name);
    output::config("sha", &sha);

    Ok(PushConfig { registry, version, repo_url, ref_name, sha })
}

impl PushConfig {
    fn push(&self, entries: &[crate::types::BuildMatrixEntry]) -> Result<()> {
        push_artifacts(entries, &self.registry, &self.version, &self.repo_url, &self.ref_name, &self.sha)
    }
}

fn print_dist_manifests(entries: &[crate::types::BuildMatrixEntry]) -> Result<()> {
    let mut paths: Vec<_> = entries
        .iter()
        .map(|e| format!("{}/{}.yaml", crate::paths::DIST_DIR, e.target_name))
        .collect();
    paths.sort();
    for path in &paths {
        let content = std::fs::read_to_string(path)?;
        if !content.is_empty() {
            println!("---");
            print!("{}", content);
        }
    }
    Ok(())
}

pub fn execute(args: BuildArgs) -> Result<()> {
    use crate::output;
    use crate::commands::matrix;

    output::header("Build");

    let push_cfg = if args.push {
        Some(resolve_push_config(&args)?)
    } else {
        None
    };

    let mut entries = if let Some(cluster_dir) = &args.cluster {
        matrix::discover_cluster_artifacts(cluster_dir)?
    } else {
        matrix::discover_matrix()?.build_matrix
    };

    if let Some(artifact_path) = &args.artifact {
        entries.retain(|e| e.artifact_path == *artifact_path);
    }

    if let Some(id) = &args.id {
        entries.retain(|e| e.component_id == *id);
    }

    if entries.is_empty() {
        bail!("No matching components found");
    }

    build_artifacts(&entries)?;

    if let Some(cfg) = push_cfg {
        cfg.push(&entries)?;
    } else {
        print_dist_manifests(&entries)?;
    }

    Ok(())
}
