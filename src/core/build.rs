//! The build pipeline shared by the CLI and the TUI.
//!
//! A [`BuildRequest`] is a fully resolved description of what to build (no
//! prompting, no defaults left open). [`run`] executes the seven stages and
//! reports progress through [`Reporter`], which is what lets `htmltoapk make`
//! and the TUI build screen use the very same code path.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::core::assets::{self, SourceImage};
use crate::core::config::{BuildType, Config, Signing};
use crate::core::input::WebInput;
use crate::core::process::{self, Cmd};
use crate::core::report::Reporter;
use crate::core::{doctor, fsx, naming, paths, workspace, zipper};
use crate::error::{Error, Result};

/// Human labels of the pipeline stages, also used for the TUI gauge.
pub const STAGES: [&str; 7] = [
    "preflight",
    "workspace",
    "dependencies",
    "android platform",
    "android configuration",
    "gradle build",
    "package",
];

/// Everything needed for one build. Created from [`Config`] plus explicit input.
#[derive(Debug, Clone)]
pub struct BuildRequest {
    pub input: WebInput,
    pub output: PathBuf,
    pub app_name: String,
    pub app_id: String,
    pub build_type: BuildType,

    pub workspace_root: PathBuf,
    pub keep_workspace: bool,
    pub zip_workspace: bool,
    pub offline: bool,

    pub icon: Option<SourceImage>,
    pub splash: Option<SourceImage>,
    pub background_color: String,
    pub generate_assets: bool,

    pub signing: Signing,
    pub keystore_path: Option<PathBuf>,
    pub keystore_alias: Option<String>,
    pub store_password_env: String,
    pub key_password_env: String,

    pub min_sdk: u32,
    pub target_sdk: u32,
    pub compile_sdk: u32,
    pub version_name: String,
    pub version_code: u32,

    pub capacitor_version: String,
    pub capacitor_web_dir: String,
    pub android_scheme: String,
    pub allow_mixed_content: bool,
    pub capacitor_plugins: Vec<String>,

    pub npm_registry: Option<String>,
    pub npm_extra_args: Vec<String>,
}

impl BuildRequest {
    /// Build a request from the configuration and the already resolved values.
    pub fn new(
        config: &Config,
        input: WebInput,
        output: PathBuf,
        app_name: String,
        app_id: String,
        build_type: BuildType,
    ) -> Self {
        BuildRequest {
            input,
            output: paths::absolute(&paths::expand_tilde(&output)),
            app_name,
            app_id,
            build_type,

            workspace_root: config.workspace_root(),
            keep_workspace: config.keep_workspace,
            zip_workspace: false,
            offline: config.npm.offline,

            icon: None,
            splash: None,
            background_color: config.assets.background_color.clone(),
            generate_assets: config.assets.generate,

            signing: config.signing,
            keystore_path: config
                .keystore
                .path
                .as_ref()
                .map(|path| paths::expand_tilde(path)),
            keystore_alias: config.keystore.alias.clone(),
            store_password_env: config.keystore.store_password_env.clone(),
            key_password_env: config.keystore.key_password_env.clone(),

            min_sdk: config.android.min_sdk,
            target_sdk: config.android.target_sdk,
            compile_sdk: config.android.compile_sdk,
            version_name: config.android.version_name.clone(),
            version_code: config.android.version_code,

            capacitor_version: config.capacitor.version.clone(),
            capacitor_web_dir: config.capacitor.web_dir.clone(),
            android_scheme: config.capacitor.android_scheme.clone(),
            allow_mixed_content: config.capacitor.allow_mixed_content,
            capacitor_plugins: config.capacitor.plugins.clone(),

            npm_registry: config.npm.registry.clone(),
            npm_extra_args: config.npm.extra_args.clone(),
        }
    }

    /// Filesystem-safe project slug used for the workspace and log names.
    pub fn slug(&self) -> String {
        let slug = naming::slug(&self.app_name);
        if slug.is_empty() {
            "app".to_string()
        } else {
            slug
        }
    }

    /// Directory of the generated Capacitor workspace.
    pub fn workspace_dir(&self) -> PathBuf {
        self.workspace_root.join(self.slug())
    }

    /// Final APK destination.
    pub fn output_path(&self) -> PathBuf {
        if self.output.is_dir() {
            self.output
                .join(naming::apk_file_name(&self.app_name, self.build_type.as_str()))
        } else {
            self.output.clone()
        }
    }

    /// Reject obviously broken requests before touching the filesystem.
    pub fn validate(&self) -> Result<()> {
        if !naming::is_valid_app_id(&self.app_id) {
            return Err(Error::with_hint(
                format!("`{}` is not a valid Android application id", self.app_id),
                "Use at least two lowercase segments, e.g. com.user.myapp.",
            ));
        }
        if self.app_name.trim().is_empty() {
            return Err(Error::with_hint(
                "the application name is empty",
                "Pass --name \"My App\" or set appName in the configuration.",
            ));
        }
        if self.capacitor_web_dir.trim().is_empty() {
            return Err(Error::config(
                "capacitor.webDir must not be empty (run: htmltoapk config set capacitor.webDir www)",
            ));
        }
        if self.signing == Signing::Keystore && self.build_type == BuildType::Release {
            let path = self.keystore_path.as_ref().ok_or_else(|| {
                Error::config(
                    "signing is set to `keystore` but keystore.path is unset (run: htmltoapk config set keystore.path ~/keys/release.jks)",
                )
            })?;
            if !path.is_file() {
                return Err(Error::InputNotFound { path: path.clone() });
            }
            if self.keystore_alias.is_none() {
                return Err(Error::config(
                    "signing is set to `keystore` but keystore.alias is unset (run: htmltoapk config set keystore.alias upload)",
                ));
            }
        }
        Ok(())
    }
}

/// Result of a successful build.
#[derive(Debug, Clone)]
pub struct BuildOutcome {
    pub apk: PathBuf,
    pub apk_size: u64,
    pub workspace: PathBuf,
    pub log: PathBuf,
    pub seconds: u64,
    pub zip: Option<PathBuf>,
}

/// Default APK path for an app name and variant.
pub fn default_output(config: &Config, app_name: &str, build_type: BuildType) -> PathBuf {
    config
        .output_root()
        .join(naming::apk_file_name(app_name, build_type.as_str()))
}

/// Execute the full pipeline.
pub fn run(request: &BuildRequest, reporter: &mut dyn Reporter) -> Result<BuildOutcome> {
    request.validate()?;
    let started = Instant::now();
    let total = STAGES.len();
    let log = paths::new_log_file(&request.slug())?;
    reporter.info(&format!("Log: {}", log.display()));

    // 1 - preflight -------------------------------------------------------
    reporter.stage(1, total, STAGES[0]);
    let report = doctor::run();
    report.require_build_tools()?;
    let sdk_root = doctor::android_sdk_root().ok_or_else(|| Error::AndroidSdk {
        reason: "neither ANDROID_HOME nor ANDROID_SDK_ROOT points at an Android SDK".to_string(),
        hint: doctor::ANDROID_SDK_HINT.to_string(),
    })?;
    reporter.info(&format!("Android SDK: {}", sdk_root.display()));
    reporter.info(&format!(
        "{} \u{2192} {} ({})",
        request.input.kind_label(),
        request.app_id,
        request.build_type.as_str()
    ));

    // 2 - workspace -------------------------------------------------------
    reporter.stage(2, total, STAGES[1]);
    let workspace = workspace::scaffold(request)?;
    reporter.info(&format!("Workspace: {}", workspace.root.display()));
    assets::apply(
        &workspace.root,
        request.icon.as_ref(),
        request.splash.as_ref(),
        &request.background_color,
        request.generate_assets,
        &log,
        reporter,
    )?;

    // 3 - dependencies ----------------------------------------------------
    reporter.stage(3, total, STAGES[2]);
    install_dependencies(request, &workspace.root, &log, reporter)?;

    // 4 - android platform ------------------------------------------------
    reporter.stage(4, total, STAGES[3]);
    add_or_sync_android(&workspace, &log, reporter)?;

    // 5 - android configuration -------------------------------------------
    reporter.stage(5, total, STAGES[4]);
    configure_android(request, &workspace, &sdk_root, reporter)?;

    // 6 - gradle build ----------------------------------------------------
    reporter.stage(6, total, STAGES[5]);
    run_gradle(request, &workspace, &log, reporter)?;

    // 7 - package ---------------------------------------------------------
    reporter.stage(7, total, STAGES[6]);
    let built = workspace::find_apk(&workspace, request.build_type.as_str())?;
    let destination = request.output_path();
    if let Some(parent) = destination.parent() {
        if !parent.as_os_str().is_empty() {
            fsx::create_dir_all(parent)?;
        }
    }
    fsx::copy_file(&built, &destination)?;
    let apk_size = fsx::size_of(&destination);
    reporter.success(&format!(
        "APK: {} ({})",
        destination.display(),
        fsx::human_size(apk_size)
    ));

    let mut zip = None;
    if request.zip_workspace {
        let archive = destination.with_extension("workspace.zip");
        zip = Some(zipper::zip_dir(&workspace.root, &archive)?);
        reporter.info(&format!("Workspace archived: {}", archive.display()));
    }

    if !request.keep_workspace {
        fsx::remove_dir_all(&workspace.root)?;
        reporter.info("Workspace removed (keepWorkspace = false)");
    }

    Ok(BuildOutcome {
        apk: destination,
        apk_size,
        workspace: workspace.root,
        log,
        seconds: started.elapsed().as_secs(),
        zip,
    })
}

fn install_dependencies(
    request: &BuildRequest,
    root: &Path,
    log: &Path,
    reporter: &mut dyn Reporter,
) -> Result<()> {
    let mut cmd = Cmd::new("npm")
        .arg("install")
        .arg("--no-audit")
        .arg("--no-fund")
        .cwd(root);
    if request.offline {
        cmd = cmd.arg("--offline");
    }
    if let Some(registry) = &request.npm_registry {
        cmd = cmd.arg(format!("--registry={registry}"));
    }
    cmd = cmd.args(request.npm_extra_args.clone());

    reporter.info("Installing Capacitor dependencies (npm install)");
    process::run_logged(&cmd, log, "dependencies")?;
    reporter.success("Dependencies installed");
    Ok(())
}

fn add_or_sync_android(
    workspace: &workspace::Workspace,
    log: &Path,
    reporter: &mut dyn Reporter,
) -> Result<()> {
    if workspace.has_android() {
        reporter.info("Syncing the existing Android project (npx cap sync android)");
        let cmd = Cmd::new("npx")
            .arg("--no-install")
            .arg("cap")
            .arg("sync")
            .arg("android")
            .cwd(&workspace.root);
        process::run_logged(&cmd, log, "android platform")?;
    } else {
        reporter.info("Adding the Android platform (npx cap add android)");
        let add = Cmd::new("npx")
            .arg("--no-install")
            .arg("cap")
            .arg("add")
            .arg("android")
            .cwd(&workspace.root);
        process::run_logged(&add, log, "android platform")?;
        let copy = Cmd::new("npx")
            .arg("--no-install")
            .arg("cap")
            .arg("copy")
            .arg("android")
            .cwd(&workspace.root);
        process::run_logged(&copy, log, "android platform")?;
    }
    reporter.success("Android project ready");
    Ok(())
}

fn configure_android(
    request: &BuildRequest,
    ws: &workspace::Workspace,
    sdk_root: &Path,
    reporter: &mut dyn Reporter,
) -> Result<()> {
    workspace::write_local_properties(ws, sdk_root)?;
    workspace::apply_sdk_levels(ws, request.min_sdk, request.target_sdk, request.compile_sdk)?;
    workspace::apply_app_metadata(
        ws,
        &request.app_name,
        &request.version_name,
        request.version_code,
    )?;
    reporter.info(&format!(
        "SDK levels: min {} / target {} / compile {}",
        request.min_sdk, request.target_sdk, request.compile_sdk
    ));

    match (request.build_type, request.signing) {
        (BuildType::Release, Signing::Keystore) => {
            let (store_password, key_password) = workspace::keystore_credentials(
                request.signing,
                &request.store_password_env,
                &request.key_password_env,
            )?;
            let keystore = request.keystore_path.clone().ok_or_else(|| {
                Error::config(
                    "keystore.path is unset (run: htmltoapk config set keystore.path ~/keys/release.jks)",
                )
            })?;
            let alias = request.keystore_alias.clone().unwrap_or_default();
            workspace::apply_release_signing(
                ws,
                &keystore,
                &alias,
                &store_password,
                &key_password,
            )?;
            reporter.success(&format!("Release signing via {}", keystore.display()));
        }
        (BuildType::Release, _) => {
            reporter.warn(
                "Release build without a keystore: the APK will be unsigned and cannot be installed",
            );
        }
        (BuildType::Debug, _) => {
            reporter.info("Debug signing (Android debug keystore)");
        }
    }
    Ok(())
}

fn run_gradle(
    request: &BuildRequest,
    ws: &workspace::Workspace,
    log: &Path,
    reporter: &mut dyn Reporter,
) -> Result<()> {
    let gradlew = ws.gradlew();
    if !gradlew.is_file() {
        return Err(Error::with_hint(
            format!("the Gradle wrapper is missing at {}", gradlew.display()),
            "Delete the workspace and rebuild so Capacitor can regenerate it.",
        ));
    }
    fsx::make_executable(&gradlew)?;

    let task = request.build_type.gradle_task();
    reporter.info(&format!("Running ./gradlew {task} (this can take a while)"));
    let mut cmd = Cmd::new(&gradlew)
        .arg(task)
        .arg("--console=plain")
        .cwd(&ws.android);
    if let Some(java_home) = std::env::var_os("JAVA_HOME") {
        cmd = cmd.env("JAVA_HOME", java_home);
    }
    process::run_logged(&cmd, log, "gradle build")?;
    reporter.success("Gradle build finished");
    Ok(())
}
