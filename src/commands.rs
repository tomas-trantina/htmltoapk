//! CLI command dispatch. Thin glue between clap and `crate::core`.

use std::path::PathBuf;

use crate::cli::{BuildOptions, Cli, Command, ConfigAction};
use crate::core::assets::SourceImage;
use crate::core::build::{self, BuildRequest};
use crate::core::clean::{self, CleanTargets};
use crate::core::config::{BuildType, Config};
use crate::core::input::WebInput;
use crate::core::{doctor, fsx, naming, paths, zipper};
use crate::error::{Error, Result};
use crate::ui::cli as out;

/// Execute the parsed command line.
pub fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Command::Setup { force, yes }) => setup(force, yes),
        Some(Command::Config { action }) => config(action.unwrap_or(ConfigAction::Show)),
        Some(Command::Make {
            input,
            output,
            options,
        }) => make(WebInput::resolve(&input)?, output, options),
        Some(Command::MakeDir {
            dir,
            output,
            options,
        }) => make(WebInput::resolve_dir(&dir)?, output, options),
        Some(Command::Doctor { json }) => doctor_command(json),
        Some(Command::Clean {
            workspaces,
            logs,
            dry_run,
            yes,
        }) => clean_command(workspaces, logs, dry_run, yes),
        Some(Command::Zip { path, output }) => zip_command(path, output),
        // No subcommand: the TUI is launched from main.rs.
        None => crate::ui::tui::run(),
    }
}

fn setup(force: bool, yes: bool) -> Result<()> {
    let path = Config::path()?;
    if path.is_file() && !force {
        let overwrite = if yes || !out::is_interactive() {
            false
        } else {
            out::confirm(
                &format!("Configuration already exists at {}. Overwrite?", path.display()),
                false,
            )?
        };
        if !overwrite {
            println!(
                "\nKeeping the existing configuration at {}.\nUse `htmltoapk setup --force` to recreate it.\n",
                path.display()
            );
            return config(ConfigAction::Show);
        }
    }

    let mut config_value = Config::load().unwrap_or_default();
    if !yes && out::is_interactive() {
        let prefix = out::prompt(
            "Application id prefix (e.g. com.user):",
            Some(&config_value.app_id_prefix),
        )?;
        config_value.set("appIdPrefix", &prefix)?;

        let name = out::prompt("Default application name:", Some(&config_value.app_name))?;
        config_value.set("appName", &name)?;

        let build_type = out::prompt(
            "Default build type (debug/release):",
            Some(config_value.build_type.as_str()),
        )?;
        config_value.set("buildType", &build_type)?;

        let workspace = out::prompt(
            "Workspace directory:",
            Some(&config_value.workspace.display().to_string()),
        )?;
        config_value.set("workspace", &workspace)?;

        let output_dir = out::prompt(
            "Default output directory for APKs:",
            Some(&config_value.output_dir.display().to_string()),
        )?;
        config_value.set("outputDir", &output_dir)?;

        let signing = out::prompt(
            "Signing preference (none/debug/keystore):",
            Some(config_value.signing.as_str()),
        )?;
        config_value.set("signing", &signing)?;
    }

    let written = config_value.save()?;
    fsx::create_dir_all(&config_value.workspace_root())?;

    out::print_success(
        "configuration created",
        &[
            ("config", written.display().to_string()),
            ("workspace", config_value.workspace_root().display().to_string()),
            ("prefix", config_value.app_id_prefix.clone()),
            ("variant", config_value.build_type.to_string()),
        ],
    );

    let report = doctor::run();
    out::print_doctor(&report);
    Ok(())
}

fn config(action: ConfigAction) -> Result<()> {
    let path = Config::path()?;
    match action {
        ConfigAction::Show => {
            let config = Config::load()?;
            out::print_config(&config, &path, path.is_file());
        }
        ConfigAction::Get { key } => {
            let config = Config::load()?;
            println!("{}", config.get(&key)?);
        }
        ConfigAction::Set { key, value } => {
            let mut config = Config::load()?;
            config.set(&key, &value)?;
            let written = config.save()?;
            out::print_success(
                "configuration updated",
                &[
                    (&key, config.get(&key)?),
                    ("file", written.display().to_string()),
                ],
            );
        }
        ConfigAction::Path => println!("{}", path.display()),
        ConfigAction::Reset { yes } => {
            if !yes && out::is_interactive() && !out::confirm("Restore every default value?", false)? {
                return Err(Error::Cancelled);
            }
            let config = Config::default();
            let written = config.save()?;
            out::print_success(
                "configuration reset",
                &[("file", written.display().to_string())],
            );
        }
        ConfigAction::Dump => {
            let config = Config::load()?;
            print!("{}", config.to_toml()?);
        }
    }
    Ok(())
}

fn make(input: WebInput, output: Option<PathBuf>, options: BuildOptions) -> Result<()> {
    let config = Config::load()?;
    let interactive = !options.yes && out::is_interactive();

    // Application name: flag -> prompt -> derived -> config default.
    let derived_name = input.suggested_app_name();
    let mut app_name = config.resolve_app_name(options.name.as_deref(), Some(&derived_name));
    if options.name.is_none() && interactive {
        app_name = out::prompt("Application name:", Some(&app_name))?;
    }

    // Application id.
    let mut app_id = config.resolve_app_id(options.app_id.as_deref(), &app_name)?;
    if options.app_id.is_none() && interactive {
        let answer = out::prompt("Application id:", Some(&app_id))?;
        app_id = config.resolve_app_id(Some(&answer), &app_name)?;
    }

    // Build variant.
    let build_type = match &options.build_type {
        Some(value) => BuildType::parse(value)?,
        None => config.build_type,
    };

    // Output path.
    let output = match output {
        Some(path) => path,
        None => {
            let default = build::default_output(&config, &app_name, build_type);
            if interactive {
                PathBuf::from(out::prompt(
                    "Output APK path:",
                    Some(&default.display().to_string()),
                )?)
            } else {
                default
            }
        }
    };

    let mut request = BuildRequest::new(&config, input, output, app_name, app_id, build_type);
    request.zip_workspace = options.zip;
    if options.offline {
        request.offline = true;
    }
    if options.keep_workspace {
        request.keep_workspace = true;
    }
    if options.discard_workspace {
        request.keep_workspace = false;
    }

    let icon = options.icon.clone().or_else(|| config.assets.icon.clone());
    if let Some(icon) = icon {
        request.icon = Some(SourceImage::resolve(&paths::expand_tilde(&icon), "icon")?);
    }
    let splash = options.splash.clone().or_else(|| config.assets.splash.clone());
    if let Some(splash) = splash {
        request.splash = Some(SourceImage::resolve(&paths::expand_tilde(&splash), "splash")?);
    }

    let mut reporter = out::ConsoleReporter::new();
    let outcome = build::run(&request, &mut reporter)?;

    let mut lines = vec![
        ("apk", outcome.apk.display().to_string()),
        ("size", fsx::human_size(outcome.apk_size)),
        ("variant", request.build_type.to_string()),
        ("app id", request.app_id.clone()),
        ("took", format!("{}s", outcome.seconds)),
        ("log", outcome.log.display().to_string()),
    ];
    if request.keep_workspace {
        lines.push(("workspace", outcome.workspace.display().to_string()));
    }
    if let Some(zip) = &outcome.zip {
        lines.push(("archive", zip.display().to_string()));
    }
    out::print_success(&format!("{} is ready", request.app_name), &lines);
    Ok(())
}

fn doctor_command(json: bool) -> Result<()> {
    let report = doctor::run();
    if json {
        println!("{}", report.to_json());
    } else {
        out::print_doctor(&report);
    }
    if report.is_ok() {
        Ok(())
    } else {
        report.require_build_tools()
    }
}

fn clean_command(workspaces: bool, logs: bool, dry_run: bool, yes: bool) -> Result<()> {
    let config = Config::load()?;
    let mut report = clean::scan(&config, CleanTargets { workspaces, logs })?;
    report.dry_run = dry_run;

    if report.is_empty() {
        println!("\nNothing to clean.\n");
        return Ok(());
    }

    println!("\nRemovable items:");
    for candidate in &report.candidates {
        println!(
            "  {:<9} {}  ({})",
            candidate.label,
            candidate.path.display(),
            fsx::human_size(candidate.bytes)
        );
    }
    println!(
        "\nTotal: {} in {} item(s)\n",
        fsx::human_size(report.total_bytes()),
        report.candidates.len()
    );

    if dry_run {
        println!("Dry run: nothing was deleted.\n");
        return Ok(());
    }
    if !yes && out::is_interactive() && !out::confirm("Delete the items above?", false)? {
        return Err(Error::Cancelled);
    }

    clean::remove(&mut report)?;
    out::print_success(
        "cleanup finished",
        &[
            ("removed", report.removed.len().to_string()),
            ("freed", fsx::human_size(report.total_bytes())),
        ],
    );
    Ok(())
}

fn zip_command(path: Option<PathBuf>, output: Option<PathBuf>) -> Result<()> {
    let source = paths::absolute(&path.unwrap_or_else(|| PathBuf::from(".")));
    if !source.exists() {
        return Err(Error::InputNotFound { path: source });
    }
    let destination = match output {
        Some(path) => path,
        None => PathBuf::from(format!("{}.zip", naming::slug(
            &source
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "archive".to_string()),
        ))),
    };
    let archive = zipper::zip_dir(&source, &paths::absolute(&destination))?;
    out::print_success(
        "archive created",
        &[
            ("archive", archive.display().to_string()),
            ("size", fsx::human_size(fsx::size_of(&archive))),
        ],
    );
    Ok(())
}
