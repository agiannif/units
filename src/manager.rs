use anyhow::{Result, anyhow, bail};
use std::{env, fs, path, process};

use crate::app::App;
use crate::logging;

pub struct Manager {
    repo_dir: path::PathBuf,
    force: bool,
    dry_run: bool,
}

impl Manager {
    pub fn new(force: bool, dry_run: bool) -> Result<Self> {
        check_root()?;

        let exe_path = env::current_exe()?;
        let repo_dir = exe_path
            .parent()
            .ok_or_else(|| anyhow!("Failed to find script directory"))?
            .to_path_buf();

        Ok(Manager {
            repo_dir,
            force,
            dry_run,
        })
    }

    pub fn status(&self, app_name: Option<String>) -> Result<()> {
        match app_name {
            Some(app_name) => {
                let app = App::new(&app_name)?;
                let status = app.get_status()?;
                logging::info(&format!("Status for {}: {status}", app.name))
            }
            None => {
                let apps = self.discover_apps()?;
                if apps.is_empty() {
                    logging::warn("No apps found");
                    return Ok(());
                }

                for app in apps {
                    let status = app.get_status()?;
                    logging::info(&format!("Status for {}: {status}", app.name))
                }
            }
        }
        Ok(())
    }

    pub fn install_apps(&self, app_name: Option<String>) -> Result<()> {
        match app_name {
            Some(app_name) => {
                let app = App::new(&app_name)?;
                app.install(self.dry_run, self.force)?;
                logging::success(&format!("App {} installed and started", app.name));
            }
            None => {
                let apps = self.discover_apps()?;
                if apps.is_empty() {
                    logging::warn("No apps found");
                    return Ok(());
                }

                for app in apps {
                    logging::info(&format!("Installing app {}", app.name));
                    app.install(self.dry_run, self.force)?;
                    logging::success(&format!("App {} installed and started", app.name));
                }
            }
        }
        Ok(())
    }

    pub fn uninstall_apps(&self, app_name: Option<String>) -> Result<()> {
        match app_name {
            Some(app_name) => {
                let app = App::new(&app_name)?;
                app.uninstall(self.dry_run, self.force)?;
                logging::success(&format!("App {} uninstalled", app.name));
            }
            None => {
                let apps = self.discover_apps()?;
                if apps.is_empty() {
                    logging::warn("No apps found");
                }

                for app in apps {
                    logging::info(&format!("Uninstalling app {}", app.name));
                    app.uninstall(self.dry_run, self.force)?;
                    logging::success(&format!("App {} uninstalled", app.name));
                }
            }
        }
        Ok(())
    }

    pub fn show_logs(&self, app_name: String) -> Result<()> {
        let app = App::new(&app_name)?;

        logging::info(&format!(
            "Showing logs for {app_name} (Press Ctrl+C to exit)"
        ));
        app.logs()
    }

    fn discover_apps(&self) -> Result<Vec<App>> {
        let mut apps = Vec::new();

        for entry in fs::read_dir(&self.repo_dir)? {
            let path = entry?.path();

            let app_name = path.file_name().unwrap().to_str().unwrap();
            if path.is_dir() && !app_name.starts_with('.') {
                apps.push(App::new(app_name)?);
            }
        }

        Ok(apps)
    }
}

fn check_root() -> Result<()> {
    let output = process::Command::new("id").arg("-u").output()?;
    let uid = String::from_utf8(output.stdout)?.trim().parse::<u32>()?;

    if uid != 0 {
        bail!("This script must be run as root (for systemd operations)");
    }
    Ok(())
}
