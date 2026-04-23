mod utils;
mod overlays;
mod rendering;
mod artifacts;
mod push;

use anyhow::{Result, bail};

pub use utils::*;
pub(crate) use artifacts::build_single_artifact;
pub use artifacts::{build_all_artifacts, build_cluster_artifacts};
pub use push::push_artifacts;

pub fn validate_build(artifact_path: &str, cluster_path: &str) -> Result<bool> {
    match build_single_artifact(artifact_path, cluster_path, false) {
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

pub fn execute(args: BuildArgs) -> Result<()> {
    use crate::output;

    output::header("Build");

    let push_cfg = if args.push {
        Some(resolve_push_config(&args)?)
    } else {
        None
    };

    let build_all = args.all || (args.artifact.is_none() && args.cluster.is_none());

    if build_all {
        let matrix = build_all_artifacts()?;
        if let Some(cfg) = push_cfg {
            cfg.push(&matrix.build_matrix)?;
        }
    } else if args.artifact.is_none() {
        let cluster_dir = args.cluster.unwrap();
        let entries = build_cluster_artifacts(&cluster_dir)?;
        if let Some(cfg) = push_cfg {
            cfg.push(&entries)?;
        }
    } else {
        let artifact_path = args.artifact.unwrap();
        let cluster_dir = args.cluster
            .ok_or_else(|| anyhow::anyhow!("--cluster is required when --artifact is provided"))?;

        let output_content = build_single_artifact(&artifact_path, &cluster_dir, args.push)?;

        if let Some(cfg) = push_cfg {
            let cluster_name = get_cluster_name(&cluster_dir)?;
            let artifact_name = std::path::Path::new(&artifact_path)
                .file_name()
                .unwrap()
                .to_string_lossy();
            let target_name = format!("{}-{}", artifact_name, cluster_name);

            cfg.push(&[crate::types::BuildMatrixEntry {
                artifact_path,
                cluster: cluster_dir,
                cluster_name,
                target_name,
            }])?;
        } else {
            println!("{}", output_content);
        }
    }

    Ok(())
}
