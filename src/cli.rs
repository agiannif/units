use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,

    /// Skip confirmations
    #[arg(long)]
    pub force: bool,

    /// Show plan without executing
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show status of apps
    Status { app_name: Option<String> },
    /// Install an app
    Install { app_name: Option<String> },
    /// Uninstall an app
    Uninstall { app_name: Option<String> },
    /// Show logs for an app
    Logs { app_name: String },
}
