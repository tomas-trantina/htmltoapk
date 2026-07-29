//! Well-known filesystem locations (XDG-friendly).

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::fsx;
use crate::error::{Error, Result};

fn home() -> Option<PathBuf> {
    dirs::home_dir()
}

/// `~/.config/htmltoapk`
pub fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| {
        Error::with_hint(
            "could not determine the user configuration directory",
            "Set $HOME or $XDG_CONFIG_HOME and try again.",
        )
    })?;
    Ok(base.join("htmltoapk"))
}

/// `~/.config/htmltoapk/config.toml`
pub fn config_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// `~/.local/share/htmltoapk`
pub fn data_dir() -> Result<PathBuf> {
    let base = dirs::data_dir().ok_or_else(|| {
        Error::with_hint(
            "could not determine the user data directory",
            "Set $HOME or $XDG_DATA_HOME and try again.",
        )
    })?;
    Ok(base.join("htmltoapk"))
}

/// Default root for generated Capacitor workspaces.
pub fn default_workspace_root() -> PathBuf {
    data_dir()
        .map(|dir| dir.join("workspaces"))
        .unwrap_or_else(|_| PathBuf::from(".htmltoapk/workspaces"))
}

/// Directory holding build logs.
pub fn log_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("logs"))
}

/// Seconds since the Unix epoch (used for log file names).
pub fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Create and return a fresh log file path for a build.
pub fn new_log_file(slug: &str) -> Result<PathBuf> {
    let dir = log_dir()?;
    fsx::create_dir_all(&dir)?;
    Ok(dir.join(format!("{slug}-{}.log", unix_secs())))
}

/// Expand a leading `~` in user-provided paths.
pub fn expand_tilde(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if text == "~" {
        return home().unwrap_or_else(|| path.to_path_buf());
    }
    if let Some(rest) = text.strip_prefix("~/") {
        if let Some(home) = home() {
            return home.join(rest);
        }
    }
    path.to_path_buf()
}

/// Absolute path without requiring the file to exist.
pub fn absolute(path: &Path) -> PathBuf {
    let expanded = expand_tilde(path);
    if expanded.is_absolute() {
        return expanded;
    }
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(expanded),
        Err(_) => expanded,
    }
}

/// Render a path with `~` for nicer UI output.
pub fn pretty(path: &Path) -> String {
    if let Some(home) = home() {
        if let Ok(rest) = path.strip_prefix(&home) {
            return format!("~/{}", rest.display());
        }
    }
    path.display().to_string()
}
