//! htmltoapk: turn HTML websites into Android APKs.
//!
//! Architecture:
//! - `core`   pure logic (config, input, workspace, build, doctor, clean, zip)
//! - `ui`     front-ends (`ui::cli` output/prompts, `ui::tui` interactive app)
//! - `cli`    clap definitions, `commands` dispatch
//! - `error`  one error type with headlines, hints and exit codes
//!
//! Running the binary without arguments opens the TUI.

mod cli;
mod commands;
mod core;
mod error;
mod ui;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    // No arguments: open the interactive interface.
    if std::env::args_os().len() <= 1 {
        return match ui::tui::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                ui::cli::print_error(&error);
                ExitCode::from(error.exit_code() as u8)
            }
        };
    }

    let parsed = match cli::Cli::try_parse() {
        Ok(parsed) => parsed,
        Err(error) => {
            // clap already renders help and usage errors nicely.
            let _ = error.print();
            return ExitCode::from(error.exit_code() as u8);
        }
    };

    match commands::dispatch(parsed) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            ui::cli::print_error(&error);
            ExitCode::from(error.exit_code() as u8)
        }
    }
}
