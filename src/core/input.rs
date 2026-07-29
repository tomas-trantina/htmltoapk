//! Input resolution: single-file HTML documents and web directories.

use std::path::{Path, PathBuf};

use crate::core::{fsx, naming, paths};
use crate::error::{Error, Result};

/// What the user pointed us at.
#[derive(Debug, Clone)]
pub enum WebInput {
    /// One self-contained `.html` file.
    SingleFile { file: PathBuf },
    /// A directory of web assets with an entry point inside it.
    Directory { root: PathBuf, entry: PathBuf },
}

impl WebInput {
    /// Resolve any path into a usable web input.
    pub fn resolve(path: &Path) -> Result<Self> {
        let path = paths::absolute(path);
        if !path.exists() {
            return Err(Error::InputNotFound { path });
        }
        if path.is_dir() {
            return Self::resolve_dir(&path);
        }
        let extension = path
            .extension()
            .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if extension != "html" && extension != "htm" {
            return Err(Error::InvalidInput {
                path,
                reason: format!("expected a .html file, got `.{extension}`"),
            });
        }
        Ok(WebInput::SingleFile { file: path })
    }

    /// Resolve a directory input (used by `make-dir`).
    pub fn resolve_dir(path: &Path) -> Result<Self> {
        let root = paths::absolute(path);
        if !root.exists() {
            return Err(Error::InputNotFound { path: root });
        }
        if !root.is_dir() {
            return Err(Error::InvalidInput {
                path: root,
                reason: "expected a directory".to_string(),
            });
        }
        let entry = find_entry(&root)?;
        Ok(WebInput::Directory { root, entry })
    }

    /// Path shown in the UI.
    pub fn display_path(&self) -> &Path {
        match self {
            WebInput::SingleFile { file } => file,
            WebInput::Directory { root, .. } => root,
        }
    }

    /// The HTML entry point of this input.
    pub fn entry(&self) -> &Path {
        match self {
            WebInput::SingleFile { file } => file,
            WebInput::Directory { entry, .. } => entry,
        }
    }

    /// Human label such as `single-file HTML` used in logs.
    pub fn kind_label(&self) -> &'static str {
        match self {
            WebInput::SingleFile { .. } => "single-file HTML",
            WebInput::Directory { .. } => "web directory",
        }
    }

    /// Best guess for the application name.
    pub fn suggested_app_name(&self) -> String {
        match self {
            WebInput::SingleFile { file } => title_from_html(file)
                .unwrap_or_else(|| naming::app_name_from_path(file)),
            WebInput::Directory { root, entry } => title_from_html(entry)
                .unwrap_or_else(|| naming::app_name_from_path(root)),
        }
    }

    /// Copy the web assets into the Capacitor `webDir`.
    pub fn materialize(&self, web_dir: &Path) -> Result<u64> {
        fsx::create_dir_all(web_dir)?;
        match self {
            WebInput::SingleFile { file } => {
                fsx::copy_file(file, &web_dir.join("index.html"))?;
                Ok(1)
            }
            WebInput::Directory { root, entry } => {
                let copied = fsx::copy_dir(root, web_dir, fsx::DEFAULT_SKIP)?;
                let index = web_dir.join("index.html");
                if !index.exists() {
                    // The entry point was not called index.html: promote it.
                    fsx::copy_file(entry, &index)?;
                }
                Ok(copied)
            }
        }
    }
}

/// Find an entry point inside a web directory.
fn find_entry(root: &Path) -> Result<PathBuf> {
    let index = root.join("index.html");
    if index.is_file() {
        return Ok(index);
    }
    let candidates = ["index.htm", "main.html", "app.html", "home.html"];
    for candidate in candidates {
        let path = root.join(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    // Fall back to any HTML file, preferring shallow ones.
    let files = fsx::list_files(root, fsx::DEFAULT_SKIP)?;
    let mut html: Vec<PathBuf> = files
        .into_iter()
        .filter(|path| {
            path.extension()
                .map(|ext| {
                    let ext = ext.to_string_lossy().to_ascii_lowercase();
                    ext == "html" || ext == "htm"
                })
                .unwrap_or(false)
        })
        .collect();
    html.sort_by_key(|path| (path.components().count(), path.clone()));
    match html.first() {
        Some(relative) => Ok(root.join(relative)),
        None => Err(Error::InvalidInput {
            path: root.to_path_buf(),
            reason: "no HTML entry point found (expected index.html)".to_string(),
        }),
    }
}

/// Read `<title>` from an HTML file, if it looks usable as an app name.
fn title_from_html(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let lower = text.to_lowercase();
    let start = lower.find("<title")?;
    let open_end = lower[start..].find('>')? + start + 1;
    let close = lower[open_end..].find("</title>")? + open_end;
    let title = text[open_end..close].trim();
    let title: String = title.split_whitespace().collect::<Vec<&str>>().join(" ");
    if title.is_empty() || title.len() > 40 {
        return None;
    }
    let generic = ["document", "index", "untitled", "home", "page", "html"];
    if generic.contains(&title.to_lowercase().as_str()) {
        return None;
    }
    Some(title)
}
