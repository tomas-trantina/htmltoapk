//! Environment diagnostics shared by `htmltoapk doctor` and the build preflight.

use std::path::{Path, PathBuf};

use crate::core::config::Config;
use crate::core::process;
use crate::error::{Error, Result};

/// Result of a single check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Warn,
    Fail,
}

impl Status {
    pub fn symbol(self) -> &'static str {
        match self {
            Status::Ok => "OK",
            Status::Warn => "WARN",
            Status::Fail => "FAIL",
        }
    }
}

/// One diagnostic row.
#[derive(Debug, Clone)]
pub struct Check {
    pub name: String,
    pub status: Status,
    pub detail: String,
    pub hint: Option<String>,
    /// Required for a successful APK build.
    pub required: bool,
    /// Tool identifier used to build a precise error (`node`, `npm`, `android-sdk`, ...).
    pub tool: Option<String>,
}

impl Check {
    fn ok(name: &str, detail: impl Into<String>) -> Self {
        Check {
            name: name.to_string(),
            status: Status::Ok,
            detail: detail.into(),
            hint: None,
            required: true,
            tool: None,
        }
    }

    fn warn(name: &str, detail: impl Into<String>, hint: impl Into<String>) -> Self {
        Check {
            name: name.to_string(),
            status: Status::Warn,
            detail: detail.into(),
            hint: Some(hint.into()),
            required: false,
            tool: None,
        }
    }

    fn fail(name: &str, detail: impl Into<String>, tool: &str) -> Self {
        Check {
            name: name.to_string(),
            status: Status::Fail,
            detail: detail.into(),
            hint: None,
            required: true,
            tool: Some(tool.to_string()),
        }
    }

    fn optional(mut self) -> Self {
        self.required = false;
        self
    }
}

/// Full diagnostic report.
#[derive(Debug, Clone, Default)]
pub struct Report {
    pub checks: Vec<Check>,
}

impl Report {
    /// No hard failures.
    pub fn is_ok(&self) -> bool {
        !self.checks.iter().any(|check| check.status == Status::Fail)
    }

    pub fn warnings(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.status == Status::Warn)
            .count()
    }

    pub fn failures(&self) -> Vec<&Check> {
        self.checks
            .iter()
            .filter(|check| check.status == Status::Fail)
            .collect()
    }

    /// Machine readable output for `doctor --json`.
    pub fn to_json(&self) -> String {
        let checks: Vec<serde_json::Value> = self
            .checks
            .iter()
            .map(|check| {
                serde_json::json!({
                    "name": check.name,
                    "status": check.status.symbol().to_lowercase(),
                    "detail": check.detail,
                    "hint": check.hint,
                    "required": check.required,
                })
            })
            .collect();
        let value = serde_json::json!({
            "ok": self.is_ok(),
            "warnings": self.warnings(),
            "checks": checks,
        });
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
    }

    /// Convert the first blocking failure into a precise error.
    pub fn require_build_tools(&self) -> Result<()> {
        for check in &self.checks {
            if check.status != Status::Fail || !check.required {
                continue;
            }
            let tool = check.tool.clone().unwrap_or_default();
            return Err(match tool.as_str() {
                "android-sdk" => Error::AndroidSdk {
                    reason: check.detail.clone(),
                    hint: ANDROID_SDK_HINT.to_string(),
                },
                "config" => Error::config(check.detail.clone()),
                "" => Error::other(check.detail.clone()),
                tool => Error::missing_tool(tool),
            });
        }
        Ok(())
    }
}

pub const ANDROID_SDK_HINT: &str = "Install the Android SDK and export its location:\n  \
     export ANDROID_HOME=\"$HOME/Android/Sdk\"\n  \
     export PATH=\"$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH\"\n  \
     Then install the build pieces:\n  \
     sdkmanager \"platform-tools\" \"platforms;android-34\" \"build-tools;34.0.0\"";

/// The Android SDK root, from `ANDROID_HOME` or `ANDROID_SDK_ROOT`.
pub fn android_sdk_root() -> Option<PathBuf> {
    for key in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(value) = std::env::var_os(key) {
            if !value.is_empty() {
                return Some(PathBuf::from(value));
            }
        }
    }
    let default = dirs::home_dir()?.join("Android/Sdk");
    if default.is_dir() {
        Some(default)
    } else {
        None
    }
}

fn has_children(path: &Path) -> bool {
    std::fs::read_dir(path)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false)
}

fn tool_check(name: &str, tool: &str, args: &[&str], minimum: Option<u32>) -> Check {
    match process::which(tool) {
        None => Check::fail(name, format!("`{tool}` was not found in PATH"), tool),
        Some(path) => {
            let version = process::probe(tool, args);
            match (&version, minimum) {
                (Some(text), Some(minimum)) => match process::major_version(text) {
                    Some(major) if major < minimum => Check::warn(
                        name,
                        format!("{text} ({})", path.display()),
                        format!("Version {minimum} or newer is recommended."),
                    ),
                    _ => Check::ok(name, format!("{text} ({})", path.display())),
                },
                (Some(text), None) => Check::ok(name, format!("{text} ({})", path.display())),
                (None, _) => Check::ok(name, path.display().to_string()),
            }
        }
    }
}

/// Run every diagnostic. Never fails: problems are encoded in the report.
pub fn run() -> Report {
    let mut checks = Vec::new();

    checks.push(tool_check("Node.js", "node", &["--version"], Some(18)));
    checks.push(tool_check("npm", "npm", &["--version"], Some(9)));
    checks.push(tool_check("npx", "npx", &["--version"], None));
    checks.push(tool_check("Java (JDK)", "java", &["-version"], Some(17)));

    // Android SDK.
    checks.push(match android_sdk_root() {
        None => Check::fail(
            "Android SDK",
            "neither ANDROID_HOME nor ANDROID_SDK_ROOT is set",
            "android-sdk",
        ),
        Some(root) if !root.is_dir() => Check::fail(
            "Android SDK",
            format!("`{}` does not exist", root.display()),
            "android-sdk",
        ),
        Some(root) => {
            let platform_tools = root.join("platform-tools");
            let build_tools = root.join("build-tools");
            let platforms = root.join("platforms");
            if !platform_tools.is_dir() {
                Check::fail(
                    "Android SDK",
                    format!("`platform-tools` is missing in {}", root.display()),
                    "android-sdk",
                )
            } else if !build_tools.is_dir() || !has_children(&build_tools) {
                Check::fail(
                    "Android SDK",
                    format!("no `build-tools` installed in {}", root.display()),
                    "android-sdk",
                )
            } else if !platforms.is_dir() || !has_children(&platforms) {
                Check::fail(
                    "Android SDK",
                    format!("no `platforms` installed in {}", root.display()),
                    "android-sdk",
                )
            } else {
                Check::ok("Android SDK", root.display().to_string())
            }
        }
    });

    // JAVA_HOME is not strictly required but avoids a very common Gradle failure.
    checks.push(match std::env::var_os("JAVA_HOME") {
        Some(value) if !value.is_empty() => {
            Check::ok("JAVA_HOME", PathBuf::from(value).display().to_string()).optional()
        }
        _ => Check::warn(
            "JAVA_HOME",
            "not set",
            "Gradle occasionally needs it: export JAVA_HOME=/usr/lib/jvm/java-17-openjdk",
        ),
    });

    // Optional convenience tools.
    checks.push(match process::which("git") {
        Some(path) => Check::ok("git (optional)", path.display().to_string()).optional(),
        None => Check::warn(
            "git (optional)",
            "not found",
            "Only needed if you want to version the generated workspace.",
        ),
    });

    // Configuration.
    checks.push(match Config::load() {
        Ok(config) => {
            let detail = if Config::exists() {
                Config::path()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|_| "loaded".to_string())
            } else {
                "using built-in defaults (run `htmltoapk setup`)".to_string()
            };
            let mut check = Check::ok("Configuration", detail);
            if !Config::exists() {
                check.status = Status::Warn;
                check.required = false;
                check.hint = Some("Create it with: htmltoapk setup".to_string());
            }
            let _ = config;
            check
        }
        Err(err) => Check::fail("Configuration", err.to_string(), "config"),
    });

    // Workspace writability.
    checks.push(match Config::load() {
        Ok(config) => {
            let root = config.workspace_root();
            match std::fs::create_dir_all(&root) {
                Ok(()) => Check::ok("Workspace directory", root.display().to_string()),
                Err(err) => Check::fail(
                    "Workspace directory",
                    format!("`{}` is not writable: {err}", root.display()),
                    "",
                ),
            }
        }
        Err(_) => Check::warn(
            "Workspace directory",
            "skipped (invalid configuration)",
            "Fix the configuration first.",
        ),
    });

    Report { checks }
}

/// Convenience wrapper used by the build preflight stage.
pub fn require_build_tools() -> Result<()> {
    run().require_build_tools()
}
