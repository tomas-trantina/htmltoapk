//! Small filesystem helpers that always attach human-readable context to errors.

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::error::{IoContext, Result};

/// Directory entries that are never copied into a workspace.
pub const DEFAULT_SKIP: &[&str] = &[
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    ".DS_Store",
    ".htmltoapk",
    "android",
    "target",
];

pub fn create_dir_all(path: &Path) -> Result<()> {
    fs::create_dir_all(path).ctx(format!(
        "could not create directory `{}`",
        path.display()
    ))
}

pub fn read_to_string(path: &Path) -> Result<String> {
    fs::read_to_string(path).ctx(format!("could not read `{}`", path.display()))
}

pub fn write(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            create_dir_all(parent)?;
        }
    }
    fs::write(path, contents).ctx(format!("could not write `{}`", path.display()))
}

pub fn copy_file(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        if !parent.as_os_str().is_empty() {
            create_dir_all(parent)?;
        }
    }
    fs::copy(from, to)
        .ctx(format!(
            "could not copy `{}` to `{}`",
            from.display(),
            to.display()
        ))
        .map(|_| ())
}

pub fn remove_dir_all(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    fs::remove_dir_all(path).ctx(format!("could not remove directory `{}`", path.display()))
}

pub fn remove_file(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(path).ctx(format!("could not remove file `{}`", path.display()))
}

/// Recursively copy `src` into `dst`, skipping any entry whose file name is in `skip`.
/// Returns the number of copied files.
pub fn copy_dir(src: &Path, dst: &Path, skip: &[&str]) -> Result<u64> {
    create_dir_all(dst)?;
    let mut copied = 0u64;
    let entries = fs::read_dir(src).ctx(format!("could not read directory `{}`", src.display()))?;
    for entry in entries {
        let entry = entry.ctx(format!("could not read entry in `{}`", src.display()))?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();
        if skip.iter().any(|candidate| *candidate == name_str) {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        let file_type = entry
            .file_type()
            .ctx(format!("could not inspect `{}`", from.display()))?;
        if file_type.is_dir() {
            copied += copy_dir(&from, &to, skip)?;
        } else {
            copy_file(&from, &to)?;
            copied += 1;
        }
    }
    Ok(copied)
}

/// Recursively collect every file below `root` (relative paths), skipping `skip` entries.
pub fn list_files(root: &Path, skip: &[&str]) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect(root, root, skip, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect(root: &Path, dir: &Path, skip: &[&str], out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).ctx(format!("could not read directory `{}`", dir.display()))?;
    for entry in entries {
        let entry = entry.ctx(format!("could not read entry in `{}`", dir.display()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if skip.iter().any(|candidate| *candidate == name) {
            continue;
        }
        let path = entry.path();
        let file_type = entry
            .file_type()
            .ctx(format!("could not inspect `{}`", path.display()))?;
        if file_type.is_dir() {
            collect(root, &path, skip, out)?;
        } else if let Ok(relative) = path.strip_prefix(root) {
            out.push(relative.to_path_buf());
        }
    }
    Ok(())
}

/// Total size in bytes of a file or directory tree (best effort).
pub fn size_of(path: &Path) -> u64 {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(_) => return 0,
    };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            total += size_of(&entry.path());
        }
    }
    total
}

/// Last `lines` lines of a (possibly large) text file. Never fails.
pub fn tail(path: &Path, lines: usize) -> String {
    const WINDOW: u64 = 64 * 1024;
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return String::new(),
    };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(WINDOW);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return String::new();
    }
    let mut buffer = Vec::new();
    if file.read_to_end(&mut buffer).is_err() {
        return String::new();
    }
    let text = String::from_utf8_lossy(&buffer);
    let collected: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let skip = collected.len().saturating_sub(lines);
    collected[skip..].join("\n")
}

/// Human-friendly byte size, e.g. `12.4 MB`.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Make a file executable (no-op on non-unix platforms).
pub fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path.exists() {
            let mut permissions = fs::metadata(path)
                .ctx(format!("could not read metadata of `{}`", path.display()))?
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).ctx(format!(
                "could not make `{}` executable",
                path.display()
            ))?;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_size_formats_units() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KB");
    }
}
