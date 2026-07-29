//! Icon / splash handling.
//!
//! Source images are copied into the Capacitor `resources/` convention. When
//! `assets.generate` is enabled we let `@capacitor/assets` produce every density,
//! and degrade to a warning (never a hard failure) when that is not possible.

use std::path::{Path, PathBuf};

use crate::core::process::{self, Cmd};
use crate::core::report::Reporter;
use crate::core::{fsx, paths};
use crate::error::{Error, Result};

/// A resolved, existing source image.
#[derive(Debug, Clone)]
pub struct SourceImage {
    pub path: PathBuf,
}

impl SourceImage {
    /// Validate a user supplied image path.
    pub fn resolve(path: &Path, label: &str) -> Result<Self> {
        let path = paths::absolute(path);
        if !path.exists() {
            return Err(Error::InputNotFound { path });
        }
        if !path.is_file() {
            return Err(Error::InvalidInput {
                path,
                reason: format!("{label} must be a file"),
            });
        }
        let extension = path
            .extension()
            .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "svg") {
            return Err(Error::InvalidInput {
                path,
                reason: format!("{label} must be a .png, .jpg or .svg image"),
            });
        }
        Ok(SourceImage { path })
    }
}

fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_else(|| "png".to_string())
}

/// Copy icon/splash into `<workspace>/resources` and optionally generate densities.
pub fn apply(
    workspace: &Path,
    icon: Option<&SourceImage>,
    splash: Option<&SourceImage>,
    background_color: &str,
    generate: bool,
    log: &Path,
    reporter: &mut dyn Reporter,
) -> Result<()> {
    if icon.is_none() && splash.is_none() {
        reporter.info("No custom icon or splash configured, using Capacitor defaults");
        return Ok(());
    }

    let resources = workspace.join("resources");
    fsx::create_dir_all(&resources)?;

    if let Some(icon) = icon {
        let target = resources.join(format!("icon.{}", extension_of(&icon.path)));
        fsx::copy_file(&icon.path, &target)?;
        reporter.info(&format!("Icon: {}", icon.path.display()));
    }
    if let Some(splash) = splash {
        let target = resources.join(format!("splash.{}", extension_of(&splash.path)));
        fsx::copy_file(&splash.path, &target)?;
        reporter.info(&format!("Splash: {}", splash.path.display()));
    }

    if !generate {
        reporter.info("Asset generation disabled (assets.generate = false)");
        return Ok(());
    }

    if process::which("npx").is_none() {
        reporter.warn("npx is unavailable, skipping icon/splash generation");
        return Ok(());
    }

    let cmd = Cmd::new("npx")
        .arg("--yes")
        .arg("@capacitor/assets@3")
        .arg("generate")
        .arg("--android")
        .arg("--assetPath")
        .arg("resources")
        .arg("--iconBackgroundColor")
        .arg(background_color)
        .arg("--splashBackgroundColor")
        .arg(background_color)
        .cwd(workspace);

    match process::run_logged(&cmd, log, "assets") {
        Ok(()) => {
            reporter.success("Generated launcher icons and splash screens");
            Ok(())
        }
        Err(err) => {
            reporter.warn(&format!(
                "Icon/splash generation failed, continuing with Capacitor defaults ({err})"
            ));
            Ok(())
        }
    }
}
