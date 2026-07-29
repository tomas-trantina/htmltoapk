//! External process execution with logging.
//!
//! Every command run by the build pipeline is appended to a per-build log file,
//! so failures can present the last lines of real tool output instead of a
//! generic message.

use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::core::fsx;
use crate::error::{Error, IoContext, Result};

/// A command to execute inside a working directory.
#[derive(Debug, Clone)]
pub struct Cmd {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(OsString, OsString)>,
}

impl Cmd {
    pub fn new(program: impl Into<OsString>) -> Self {
        Cmd {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        for arg in args {
            self.args.push(arg.into());
        }
        self
    }

    pub fn cwd(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cwd = Some(dir.into());
        self
    }

    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Shell-ish rendering used in logs.
    pub fn display(&self) -> String {
        let mut out = self.program.to_string_lossy().to_string();
        for arg in &self.args {
            out.push(' ');
            out.push_str(&arg.to_string_lossy());
        }
        out
    }

    fn to_command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        if let Some(dir) = &self.cwd {
            command.current_dir(dir);
        }
        for (key, value) in &self.env {
            command.env(key, value);
        }
        command
    }
}

/// Locate an executable in `PATH`.
pub fn which(tool: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(tool);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return std::fs::metadata(path)
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false);
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Run a short command and capture its first output line (used for `--version`).
pub fn probe(tool: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(tool)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let mut text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        // `java -version` writes to stderr.
        text = String::from_utf8_lossy(&output.stderr).trim().to_string();
    }
    let first = text.lines().next()?.trim().to_string();
    if first.is_empty() {
        None
    } else {
        Some(first)
    }
}

/// Extract the leading major version from a version string such as `v20.11.1`.
pub fn major_version(text: &str) -> Option<u32> {
    let mut digits = String::new();
    let mut seen_digit = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            seen_digit = true;
        } else if seen_digit {
            break;
        }
    }
    digits.parse().ok()
}

/// Run a command, streaming stdout and stderr into `log`.
///
/// On failure the returned [`Error::BuildFailed`] carries the stage name, exit
/// code, log path and the tail of the log so the UI can show real output.
pub fn run_logged(cmd: &Cmd, log: &Path, stage: &str) -> Result<()> {
    if which(&cmd.program.to_string_lossy()).is_none() && !Path::new(&cmd.program).is_file() {
        return Err(Error::missing_tool(&cmd.program.to_string_lossy()));
    }

    append(log, &format!("\n$ {}\n", cmd.display()))?;
    let out = open_log(log)?;
    let err = out.try_clone().ctx("could not duplicate the log handle")?;

    let status = cmd
        .to_command()
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .status()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                Error::missing_tool(&cmd.program.to_string_lossy())
            } else {
                Error::io(format!("could not run `{}`", cmd.display()), error)
            }
        })?;

    if status.success() {
        return Ok(());
    }

    Err(Error::BuildFailed {
        stage: stage.to_string(),
        code: status.code(),
        log: Some(log.to_path_buf()),
        tail: fsx::tail(log, 20),
    })
}

fn open_log(log: &Path) -> Result<std::fs::File> {
    if let Some(parent) = log.parent() {
        if !parent.as_os_str().is_empty() {
            fsx::create_dir_all(parent)?;
        }
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .ctx(format!("could not open log `{}`", log.display()))
}

fn append(log: &Path, text: &str) -> Result<()> {
    let mut file = open_log(log)?;
    file.write_all(text.as_bytes())
        .ctx(format!("could not write to log `{}`", log.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_major_versions() {
        assert_eq!(major_version("v20.11.1"), Some(20));
        assert_eq!(major_version("10.2.4"), Some(10));
        assert_eq!(major_version("openjdk version \"17.0.9\""), Some(17));
        assert_eq!(major_version("none"), None);
    }

    #[test]
    fn renders_commands() {
        let cmd = Cmd::new("npm").arg("install").arg("--offline");
        assert_eq!(cmd.display(), "npm install --offline");
    }
}
