use anyhow::Result;
use std::process;

pub fn is_active(service_name: &str, rootful: bool) -> Result<bool> {
    let mut args = vec!["is-active", "--quiet", service_name];
    if rootful {
        args.insert(0, "systemctl");
    } else {
        args.insert(0, "--user");
    }

    let command = if rootful { "sudo" } else { "systemctl" };
    let is_active = process::Command::new(command)
        .args(args)
        .status()
        .map(|s| s.success())?;

    Ok(is_active)
}

pub fn is_enabled(service_name: &str, rootful: bool) -> Result<bool> {
    let mut args = vec!["is-enabled", "--quiet", service_name];
    if rootful {
        args.insert(0, "systemctl");
    } else {
        args.insert(0, "--user");
    }

    let command = if rootful { "sudo" } else { "systemctl" };
    let is_active = process::Command::new(command)
        .args(args)
        .status()
        .map(|s| s.success())?;

    Ok(is_active)
}
