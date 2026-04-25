use clap::{Parser, Subcommand};
use anyhow::Result;

mod commands;
pub mod helm;
mod output;
mod paths;
mod types;

/// FedCore Platform CLI - A tool for managing Kubernetes platform deployments
#[derive(Parser)]
#[command(name = "fedcore")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate and optionally deploy bootstrap configuration for a cluster
    Bootstrap {
        /// Cluster directory (required)
        #[arg(short, long)]
        cluster: String,

        /// Deploy after generation
        #[arg(short, long)]
        deploy: bool,

        /// Generate cluster-admin prerequisite manifest (CRDs, namespace, RBAC)
        #[arg(long)]
        admin_prep: bool,

        /// OCI registry URL
        #[arg(short, long)]
        registry: Option<String>,
    },

    /// Build component artifacts for clusters
    ///
    /// Usage:
    ///   --all                    Build all components for all clusters
    ///   --cluster <path>         Build all components for a specific cluster
    ///   --artifact <path> --cluster <path>  Build a specific component for a specific cluster
    Build(commands::build::BuildArgs),

    /// Manage Helm charts: download, version discovery, updates, and OCI push
    HelmManage(commands::helm_manage::HelmManageArgs),

    /// Discover the build matrix for OCI artifacts from cluster configs
    Matrix,

    /// Query cluster status and deployed components
    Status {
        /// Kubernetes context/cluster name
        #[arg(short, long)]
        cluster: Option<String>,

        /// Namespace to query (default: all namespaces)
        #[arg(short, long)]
        namespace: Option<String>,

        /// Specific component to query
        #[arg(short = 'o', long)]
        component: Option<String>,
    },

    /// Validate ytt templates, schemas, and cluster configs
    Validate,

    /// Mirror Flux images to target registry
    MirrorFlux {
        /// Target registry URL
        #[arg(short, long)]
        registry: Option<String>,
    },

    /// Initialize a new FedCore project, cluster, or component
    Init {
        #[command(subcommand)]
        command: commands::init::InitSubcommand,
    },

    /// Compare two artifacts and show a diff between them
    ///
    /// Provide 2 targets for remote-to-remote comparison, or 1 target with
    /// --artifact and --cluster to compare a local build against a remote artifact.
    ///
    /// Each target can be an OCI reference or a deployed component name.
    Compare {
        /// Targets to compare (OCI references or deployed component names)
        #[arg(required = true, num_args = 1..=2)]
        targets: Vec<String>,

        /// Local artifact/component path for local build mode (e.g., platform/components/capsule)
        #[arg(short, long, requires = "cluster")]
        artifact: Option<String>,

        /// Local cluster directory for local build mode (e.g., platform/clusters/mycluster)
        #[arg(short, long, requires = "artifact")]
        cluster: Option<String>,

        /// Component instance ID (required when a component has multiple instances)
        #[arg(short, long)]
        id: Option<String>,

        /// Kubernetes context (for resolving deployed components)
        #[arg(long)]
        context: Option<String>,

        /// Namespace to query (for resolving deployed components)
        #[arg(short, long)]
        namespace: Option<String>,

        /// OCI registry URL
        #[arg(short, long)]
        registry: Option<String>,

        /// Registry username
        #[arg(long)]
        registry_user: Option<String>,

        /// Registry password
        #[arg(long)]
        registry_pass: Option<String>,
    },

    /// Inspect a component from an OCI artifact, local path, or deployed cluster resource
    Inspect {
        /// OCI reference, local file/directory path, or deployed component name
        target: String,

        /// Kubernetes context (for resolving deployed components)
        #[arg(short, long)]
        cluster: Option<String>,

        /// Namespace to query (for resolving deployed components)
        #[arg(short, long)]
        namespace: Option<String>,

        /// OCI registry URL
        #[arg(short, long)]
        registry: Option<String>,

        /// Registry username
        #[arg(long)]
        registry_user: Option<String>,

        /// Registry password
        #[arg(long)]
        registry_pass: Option<String>,
    },

    /// Learn how FedCore works (topics: workflow, structure, components, clusters, build, bootstrap, gitops)
    Explain {
        /// Topic to explain (omit for overview)
        topic: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    output::set_verbose(cli.verbose);

    match cli.command {
        Commands::Bootstrap { cluster, deploy, admin_prep, registry } => {
            commands::bootstrap::execute(&cluster, deploy, admin_prep, registry)?;
        }
        Commands::Build(args) => {
            commands::build::execute(args)?;
        }
        Commands::HelmManage(args) => {
            commands::helm_manage::execute(args)?;
        }
        Commands::Matrix => {
            commands::matrix::execute()?;
        }
        Commands::Status { cluster, namespace, component } => {
            commands::status::execute(cluster, namespace, component)?;
        }
        Commands::Validate => {
            commands::validate::execute()?;
        }
        Commands::MirrorFlux { registry } => {
            commands::mirror_flux::execute(registry.as_deref())?;
        }
        Commands::Compare { targets, artifact, cluster, id, context, namespace, registry, registry_user, registry_pass } => {
            commands::compare::execute(targets, artifact, cluster, id, context, namespace, registry, registry_user, registry_pass)?;
        }
        Commands::Inspect { target, cluster, namespace, registry, registry_user, registry_pass } => {
            commands::inspect::execute(&target, cluster, namespace, registry, registry_user, registry_pass)?;
        }
        Commands::Init { command } => {
            commands::init::execute(&command)?;
        }
        Commands::Explain { topic } => {
            commands::explain::execute(topic.as_deref())?;
        }
    }

    Ok(())
}
