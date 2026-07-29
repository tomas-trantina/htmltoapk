//! Command line surface (clap derive).

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

const AFTER_HELP: &str = "\
EXAMPLES:
  htmltoapk                              open the interactive TUI
  htmltoapk setup                        create the configuration file
  htmltoapk doctor                       verify node, npm, npx, java, Android SDK
  htmltoapk config                       show every configuration value
  htmltoapk config set appIdPrefix com.user
  htmltoapk make index.html app.apk      build from a single HTML file
  htmltoapk make-dir ./site site.apk     build from a web directory
  htmltoapk make ./site app.apk --build-type release --name \"My App\"
  htmltoapk clean --workspaces           free disk space
  htmltoapk zip ./site site.zip          archive a project

EXIT CODES:
  0 success   3 missing tool   4 Android SDK   5 bad input
  6 bad config   7 build failed   8 I/O error   130 cancelled
";

/// HTML websites into Android APKs, powered by Capacitor.
#[derive(Debug, Parser)]
#[command(
    name = "htmltoapk",
    version,
    about = "Turn HTML websites into Android APKs (CLI + TUI)",
    long_about = "htmltoapk packages a single-file HTML document or a web directory into an \
Android APK using Capacitor and Gradle.\n\nRun without arguments to open the interactive TUI.",
    after_help = AFTER_HELP,
    arg_required_else_help = false,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create the configuration file and check the environment.
    Setup {
        /// Overwrite an existing configuration file.
        #[arg(long)]
        force: bool,
        /// Accept defaults without asking anything.
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Show or change configuration values.
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// Build an APK from a single HTML file or a web directory.
    Make {
        /// Path to an .html file or to a web directory.
        input: PathBuf,
        /// Output .apk path (defaults to <outputDir>/<app>-<variant>.apk).
        output: Option<PathBuf>,
        #[command(flatten)]
        options: BuildOptions,
    },
    /// Build an APK from a web directory (explicit directory variant of `make`).
    #[command(name = "make-dir")]
    MakeDir {
        /// Directory containing index.html.
        dir: PathBuf,
        /// Output .apk path (defaults to <outputDir>/<app>-<variant>.apk).
        output: Option<PathBuf>,
        #[command(flatten)]
        options: BuildOptions,
    },
    /// Check that every required tool is installed.
    Doctor {
        /// Print the report as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Remove generated workspaces and build logs.
    Clean {
        /// Remove generated Capacitor workspaces.
        #[arg(long)]
        workspaces: bool,
        /// Remove build logs.
        #[arg(long)]
        logs: bool,
        /// Show what would be removed without deleting anything.
        #[arg(long)]
        dry_run: bool,
        /// Do not ask for confirmation.
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Archive a directory (or file) into a ZIP.
    Zip {
        /// Directory or file to archive (defaults to the current directory).
        path: Option<PathBuf>,
        /// Destination .zip path.
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Print every key and value (default).
    Show,
    /// Print a single value.
    Get {
        /// Configuration key, e.g. appIdPrefix or android.minSdk.
        key: String,
    },
    /// Change a single value.
    Set {
        /// Configuration key, e.g. appIdPrefix or android.minSdk.
        key: String,
        /// New value (empty string clears optional keys).
        value: String,
    },
    /// Print the configuration file path.
    Path,
    /// Restore every default value.
    Reset {
        /// Do not ask for confirmation.
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Print the configuration as TOML (useful for sharing a config).
    Dump,
}

/// Flags shared by `make` and `make-dir`.
#[derive(Debug, Args, Clone, Default)]
pub struct BuildOptions {
    /// Application name shown on the launcher.
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,
    /// Android application id, e.g. com.user.myapp.
    #[arg(long = "id", value_name = "APP_ID")]
    pub app_id: Option<String>,
    /// Build variant.
    #[arg(long = "build-type", value_name = "debug|release")]
    pub build_type: Option<String>,
    /// Source icon image (png/jpg/svg).
    #[arg(long, value_name = "FILE")]
    pub icon: Option<PathBuf>,
    /// Source splash image (png/jpg/svg).
    #[arg(long, value_name = "FILE")]
    pub splash: Option<PathBuf>,
    /// Pass --offline to npm install.
    #[arg(long)]
    pub offline: bool,
    /// Also archive the generated workspace next to the APK.
    #[arg(long)]
    pub zip: bool,
    /// Keep the generated workspace (overrides keepWorkspace).
    #[arg(long = "keep-workspace", conflicts_with = "discard_workspace")]
    pub keep_workspace: bool,
    /// Delete the generated workspace after the build.
    #[arg(long = "discard-workspace")]
    pub discard_workspace: bool,
    /// Never prompt; fail instead of asking for a missing value.
    #[arg(short = 'y', long)]
    pub yes: bool,
}
