mod templates;
mod project;
mod cluster;
mod component;

use anyhow::Result;

/// Subcommands for the init command
#[derive(Debug, Clone, clap::Subcommand)]
pub enum InitSubcommand {
    /// Initialize a new FedCore project directory structure
    Project,
    
    /// Create a new cluster configuration
    Cluster,
    
    /// Create a new component
    Component,
}

/// Execute the init command based on the subcommand
pub fn execute(subcommand: &InitSubcommand) -> Result<()> {
    match subcommand {
        InitSubcommand::Project => project::init_project(),
        InitSubcommand::Cluster => cluster::init_cluster(),
        InitSubcommand::Component => component::init_component(),
    }
}
