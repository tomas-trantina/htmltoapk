//! Housekeeping: find and delete generated Capacitor workspaces and build logs.

use std::fs;
use std::path::PathBuf;

use crate::core::config::Config;
use crate::core::{fsx, paths};
use crate::error::{IoContext, Result};

/// Which categories should be cleaned. When both flags are `false` the caller
/// asked for "everything", which [`CleanTargets::resolved`] expands.
#[derive(Debug, Clone, Copy)]
pub struct CleanTargets {
    pub workspaces: bool,
    pub logs: bool,
}

impl Default for CleanTargets {
    fn default() -> Self {
        CleanTargets {
            workspaces: true,
            logs: true,
        }
    }
}

impl CleanTargets {
    /// Expand "no flag given" into "clean everything".
    pub fn resolved(self) -> CleanTargets {
        if !self.workspaces && !self.logs {
            CleanTargets::default()
        } else {
            self
        }
    }
}

/// One removable item.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub path: PathBuf,
    pub bytes: u64,
    pub label: String,
}

/// Result of a scan, later mutated by [`remove`].
#[derive(Debug, Default)]
pub struct CleanReport {
    pub candidates: Vec<Candidate>,
    pub removed: Vec<PathBuf>,
    pub dry_run: bool,
}

impl CleanReport {
    /// Total size of all candidates.
    pub fn total_bytes(&self) -> u64 {
        self.candidates.iter().map(|item| item.bytes).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }
}

/// Collect removable workspaces and logs without touching anything.
pub fn scan(config: &Config, targets: CleanTargets) -> Result<CleanReport> {
    let targets = targets.resolved();
    let mut candidates = Vec::new();

    if targets.workspaces {
        let root = config.workspace_root();
        if root.is_dir() {
            let entries = fs::read_dir(&root)
                .ctx(format!("could not read workspace directory `{}`", root.display()))?;
            for entry in entries {
                let entry = entry.ctx(format!("could not read entry in `{}`", root.display()))?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                candidates.push(Candidate {
                    bytes: fsx::size_of(&path),
                    label: "workspace".to_string(),
                    path,
                });
            }
        }
    }

    if targets.logs {
        if let Ok(dir) = paths::log_dir() {
            if dir.is_dir() {
                let entries = fs::read_dir(&dir)
                    .ctx(format!("could not read log directory `{}`", dir.display()))?;
                for entry in entries {
                    let entry = entry.ctx(format!("could not read entry in `{}`", dir.display()))?;
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    candidates.push(Candidate {
                        bytes: fsx::size_of(&path),
                        label: "log".to_string(),
                        path,
                    });
                }
            }
        }
    }

    candidates.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    Ok(CleanReport {
        candidates,
        removed: Vec::new(),
        dry_run: false,
    })
}

/// Delete every candidate of a scan. Nothing happens for a dry run.
pub fn remove(report: &mut CleanReport) -> Result<()> {
    if report.dry_run {
        return Ok(());
    }
    for candidate in &report.candidates {
        if candidate.path.is_dir() {
            fsx::remove_dir_all(&candidate.path)?;
        } else {
            fsx::remove_file(&candidate.path)?;
        }
        report.removed.push(candidate.path.clone());
    }
    Ok(())
}
