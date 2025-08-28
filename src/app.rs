use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use std::{env, ffi, fmt, fs, path, process};
use toml;
use walkdir::WalkDir;

use crate::logging;

const CONFIG_FILE_NAME: &str = "config.toml";

pub struct App {
    pub name: String,
    app_dir: path::PathBuf,
    systemd_dir: path::PathBuf,
    use_user: bool,
}

impl App {
    pub fn new(name: &str) -> Result<Self> {
        let config_path = path::PathBuf::from(name).join(CONFIG_FILE_NAME);
        let config_str = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to find config file at {}", config_path.display()))?;
        let config: AppConfig = toml::from_str(&config_str)?;

        let exe_path = env::current_exe()?;
        let repo_dir = exe_path
            .parent()
            .ok_or_else(|| anyhow!("Failed to find current directory"))?
            .to_path_buf();
        let app_dir = repo_dir.join(name);

        Ok(App {
            name: String::from(name),
            app_dir,
            systemd_dir: path::PathBuf::from(config.systemd.install_location),
            use_user: config.systemd.use_user,
        })
    }

    pub fn get_status(&self) -> Result<AppStatus> {
        if !self.files_installed()? {
            return Ok(AppStatus::NotInstalled);
        }

        let service_name = format!("{}.service", self.name);
        let args = self.prepare_systemctl_args(vec![
            String::from("is-active"),
            String::from("--quiet"),
            service_name.clone(),
        ]);
        let is_active = process::Command::new("systemctl")
            .args(args)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if is_active {
            return Ok(AppStatus::Running);
        }

        let args = self.prepare_systemctl_args(vec![
            String::from("is-enabled"),
            String::from("--quiet"),
            service_name,
        ]);
        let is_enabled = process::Command::new("systemctl")
            .args(args)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if is_enabled {
            Ok(AppStatus::Stopped)
        } else {
            Ok(AppStatus::Installed)
        }
    }

    pub fn install(&self, dry_run: bool, force: bool) -> Result<()> {
        let app_files = self.get_app_files()?;
        if app_files.is_empty() {
            bail!("No files found for app {}", self.name)
        }

        if dry_run {
            logging::info(&format!("[DRY RUN] Would install app {}", self.name));
            for file in &app_files {
                let unit_name = file.strip_prefix(&self.app_dir)?;
                let target_path = self.systemd_dir.join(unit_name);
                logging::info(&format!(
                    "[DRY RUN] Would copy {} to {}",
                    file.to_str().unwrap(),
                    target_path.to_str().unwrap()
                ));
            }
            if self.use_user {
                logging::info(&format!(
                    "[DRY RUN] Would reload systemd and start {}.servie as user",
                    self.name
                ));
            } else {
                logging::info(&format!(
                    "[DRY RUN] Would reload systemd and start {}.service",
                    self.name
                ));
            }
            return Ok(());
        }

        // check to see if there's any collisions
        for file in &app_files {
            let unit_name = file.strip_prefix(&self.app_dir)?;
            let target_path = self.systemd_dir.join(unit_name);

            if target_path.exists() && !force {
                logging::warn(&format!(
                    "File {} already exists. Use --force to overwrite.",
                    target_path.to_str().unwrap()
                ));
                bail!("File already exsists and force not used")
            }
        }

        // copy files
        for file in &app_files {
            let unit_name = file.strip_prefix(&self.app_dir)?;
            let target_path = self.systemd_dir.join(unit_name);
            let filename = target_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();

            fs::copy(file, &target_path).context(format!(
                "Failed to copy {} to {}",
                file.to_str().unwrap(),
                target_path.to_str().unwrap(),
            ))?;
            logging::info(&format!("Copied {filename}"))
        }

        // reload systemd, start the main service
        let args = self.prepare_systemctl_args(vec![String::from("daemon-relaod")]);
        process::Command::new("systemctl").args(args).status()?;

        let service_name = format!("{}.service", self.name);
        let args = self.prepare_systemctl_args(vec![String::from("start"), service_name]);
        process::Command::new("systemctl").args(args).status()?;

        Ok(())
    }

    pub fn uninstall(&self, dry_run: bool, force: bool) -> Result<()> {
        let app_files = self.get_app_files().context("Failed to get app files")?;
        if app_files.is_empty() {
            bail!("No files found for app {}", self.name)
        }

        if dry_run {
            logging::info(&format!(
                "[DRY RUN] Would stop and disable {}.service",
                self.name
            ));

            for file in app_files {
                logging::info(&format!(
                    "[DRY RUN] Would remove {}",
                    file.to_str().unwrap()
                ));
            }

            return Ok(());
        }

        if !force {
            let confirmation = dialoguer::Confirm::new()
                .with_prompt(format!("Are you sure you want to uninstall {}?", self.name))
                .default(false)
                .interact()
                .context("Uninstall confirmation failed")?;

            if !confirmation {
                logging::info("Uninstall cancelled");
                return Ok(());
            }
        }

        // stop service if running
        let service_name = format!("{}.service", self.name);
        let args = self.prepare_systemctl_args(vec![String::from("stop"), service_name]);
        let _ = process::Command::new("systemctl").args(args).status();

        for file in app_files {
            let _ = fs::remove_file(&file);
            logging::info(&format!("Removed file {}", file.to_str().unwrap()));
        }

        // reload systemd
        let args = self.prepare_systemctl_args(vec![String::from("daemon-reload")]);
        process::Command::new("systemctl")
            .args(args)
            .status()
            .context("Failed to reload systemd after stopping service and removing files")?;

        Ok(())
    }

    pub fn logs(&self) -> Result<()> {
        let status = process::Command::new("journalctl")
            .args(["-u", &format!("{}.service", self.name), "-f"])
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to show logs for '{}'", self.name));
        }

        Ok(())
    }

    fn files_installed(&self) -> Result<bool> {
        for entry in WalkDir::new(&self.app_dir).into_iter() {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() || path.file_name() == Some(ffi::OsStr::new(CONFIG_FILE_NAME)) {
                continue;
            }

            let unit_name = path.strip_prefix(&self.app_dir)?;
            let target_path = self.systemd_dir.join(unit_name);
            if !target_path.exists() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn get_app_files(&self) -> Result<Vec<path::PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.app_dir).into_iter() {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() || path.file_name() == Some(ffi::OsStr::new(CONFIG_FILE_NAME)) {
                continue;
            }

            files.push(path.to_path_buf());
        }

        Ok(files)
    }

    fn prepare_systemctl_args(&self, mut args: Vec<String>) -> Vec<String> {
        if self.use_user {
            args.insert(0, "--user".to_string());
        }
        args
    }
}

pub enum AppStatus {
    NotInstalled,
    Installed,
    Stopped,
    Running,
}

impl fmt::Display for AppStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppStatus::NotInstalled => write!(f, "Not Installed"),
            AppStatus::Installed => write!(f, "Installed"),
            AppStatus::Stopped => write!(f, "Stopped"),
            AppStatus::Running => write!(f, "Running"),
        }
    }
}

#[derive(Deserialize)]
struct AppConfig {
    systemd: Systemd,
}

#[derive(Deserialize)]
struct Systemd {
    install_location: String,
    use_user: bool,
}
