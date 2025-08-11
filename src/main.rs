use anyhow::Result;
use clap::Parser;
use units::cli::{Args, Commands};
use units::manager::Manager;

fn main() -> Result<()> {
    let args = Args::parse();
    let manager = Manager::new(args.force, args.dry_run)?;

    match args.command {
        Commands::Status { app_name } => manager.status(app_name),
        Commands::Install { app_name } => manager.install_apps(app_name),
        Commands::Uninstall { app_name } => manager.uninstall_apps(app_name),
        Commands::Logs { app_name } => manager.show_logs(app_name),
    }
}
