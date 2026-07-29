//! ZIP export used by the `zip` command, the TUI export screen and the
//! optional `--zip` flag of a build.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::core::fsx;
use crate::error::{Error, IoContext, Result};

/// Entries that are never archived (build output and dependency caches).
pub const SKIP: &[&str] = &[
    "node_modules",
    ".git",
    ".gradle",
    ".idea",
    ".DS_Store",
    "build",
    "target",
];

/// Default archive path for a source directory: `<parent>/<name>.zip`.
pub fn default_destination(source: &Path) -> PathBuf {
    let name = source
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "archive".to_string());
    let parent = source.parent().map(Path::to_path_buf).unwrap_or_default();
    parent.join(format!("{name}.zip"))
}

/// Archive a directory (recursively) or a single file into `destination`.
///
/// Returns the archive path. The destination is created (with parents) and
/// overwritten if it already exists.
pub fn zip_dir(source: &Path, destination: &Path) -> Result<PathBuf> {
    if !source.exists() {
        return Err(Error::InputNotFound {
            path: source.to_path_buf(),
        });
    }
    if let Some(parent) = destination.parent() {
        if !parent.as_os_str().is_empty() {
            fsx::create_dir_all(parent)?;
        }
    }

    let file = File::create(destination).ctx(format!(
        "could not create archive `{}`",
        destination.display()
    ))?;
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    if source.is_file() {
        let name = source
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        add_file(&mut writer, source, &name, options)?;
    } else {
        let root_name = source
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "archive".to_string());
        let canonical_destination = destination.canonicalize().ok();
        for relative in fsx::list_files(source, SKIP)? {
            let absolute = source.join(&relative);
            // Never archive the archive we are writing right now.
            if let (Some(target), Ok(current)) = (&canonical_destination, absolute.canonicalize()) {
                if *target == current {
                    continue;
                }
            }
            let name = format!("{root_name}/{}", relative.to_string_lossy().replace('\\', "/"));
            add_file(&mut writer, &absolute, &name, options)?;
        }
    }

    writer.finish().map_err(|err| {
        Error::other(format!(
            "could not finalise archive `{}`: {err}",
            destination.display()
        ))
    })?;
    Ok(destination.to_path_buf())
}

fn add_file(
    writer: &mut ZipWriter<File>,
    path: &Path,
    name: &str,
    options: SimpleFileOptions,
) -> Result<()> {
    let bytes = std::fs::read(path).ctx(format!("could not read `{}`", path.display()))?;
    writer
        .start_file(name.to_string(), options)
        .map_err(|err| Error::other(format!("could not add `{name}` to the archive: {err}")))?;
    writer
        .write_all(&bytes)
        .ctx(format!("could not write `{name}` into the archive"))?;
    Ok(())
}
