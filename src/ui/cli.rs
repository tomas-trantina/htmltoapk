//! Terminal output and interactive prompts for the CLI front-end.

use std::io::{self, IsTerminal, Write};

use crate::core::config::Config;
use crate::core::doctor::{Report, Status};
use crate::core::report::{Level, Reporter};
use crate::error::{Error, Result};

/// ANSI styling, disabled when piping output or when `NO_COLOR` is set.
pub struct Style {
    enabled: bool,
}

impl Style {
    pub fn detect() -> Self {
        let disabled = std::env::var_os("NO_COLOR").is_some() || !io::stdout().is_terminal();
        Style { enabled: !disabled }
    }

    fn wrap(&self, code: &str, text: &str) -> String {
        if self.enabled {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    pub fn bold(&self, text: &str) -> String {
        self.wrap("1", text)
    }

    pub fn dim(&self, text: &str) -> String {
        self.wrap("2", text)
    }

    pub fn red(&self, text: &str) -> String {
        self.wrap("31;1", text)
    }

    pub fn green(&self, text: &str) -> String {
        self.wrap("32;1", text)
    }

    pub fn yellow(&self, text: &str) -> String {
        self.wrap("33;1", text)
    }

    pub fn blue(&self, text: &str) -> String {
        self.wrap("34;1", text)
    }
}

/// Streams build progress to stdout.
pub struct ConsoleReporter {
    style: Style,
    quiet: bool,
}

impl ConsoleReporter {
    pub fn new() -> Self {
        ConsoleReporter {
            style: Style::detect(),
            quiet: false,
        }
    }

    pub fn quiet() -> Self {
        ConsoleReporter {
            style: Style::detect(),
            quiet: true,
        }
    }
}

impl Default for ConsoleReporter {
    fn default() -> Self {
        ConsoleReporter::new()
    }
}

impl Reporter for ConsoleReporter {
    fn stage(&mut self, index: usize, total: usize, label: &str) {
        if self.quiet {
            return;
        }
        println!(
            "\n{} {}",
            self.style.blue(&format!("[{index}/{total}]")),
            self.style.bold(label)
        );
    }

    fn log(&mut self, level: Level, message: &str) {
        if self.quiet {
            return;
        }
        let marker = match level {
            Level::Info => self.style.dim("  ·"),
            Level::Warn => self.style.yellow("  !"),
            Level::Success => self.style.green("  ✓"),
        };
        println!("{marker} {message}");
    }
}

/// Render an error with its hint.
pub fn print_error(error: &Error) {
    let style = Style::detect();
    eprintln!("\n{} {error}", style.red("error:"));
    if let Some(hint) = error.hint() {
        eprintln!();
        for line in hint.lines() {
            eprintln!("  {line}");
        }
    }
    eprintln!();
}

/// A short success banner.
pub fn print_success(title: &str, lines: &[(&str, String)]) {
    let style = Style::detect();
    println!("\n{} {}", style.green("success:"), style.bold(title));
    let width = lines.iter().map(|(key, _)| key.len()).max().unwrap_or(0);
    for (key, value) in lines {
        println!("  {:<width$}  {value}", style.dim(key), width = width);
    }
    println!();
}

/// Ask for a line of text. Returns the default when the user just presses Enter.
pub fn prompt(question: &str, default: Option<&str>) -> Result<String> {
    let style = Style::detect();
    loop {
        match default {
            Some(value) => print!("{} {} ", style.bold(question), style.dim(&format!("[{value}]"))),
            None => print!("{} ", style.bold(question)),
        }
        io::stdout()
            .flush()
            .map_err(|err| Error::io("could not write to stdout", err))?;
        let mut line = String::new();
        let read = io::stdin()
            .read_line(&mut line)
            .map_err(|err| Error::io("could not read from stdin", err))?;
        if read == 0 {
            return Err(Error::Cancelled);
        }
        let answer = line.trim().to_string();
        if !answer.is_empty() {
            return Ok(answer);
        }
        if let Some(value) = default {
            return Ok(value.to_string());
        }
        eprintln!("  {}", style.yellow("a value is required"));
    }
}

/// Ask a yes/no question.
pub fn confirm(question: &str, default: bool) -> Result<bool> {
    let suffix = if default { "Y/n" } else { "y/N" };
    let answer = prompt(&format!("{question} ({suffix})"), Some(if default { "y" } else { "n" }))?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes" | "1" | "true"
    ))
}

/// Is stdin usable for prompting?
pub fn is_interactive() -> bool {
    io::stdin().is_terminal()
}

/// Print every configuration key and value.
pub fn print_config(config: &Config, path: &std::path::Path, exists: bool) {
    let style = Style::detect();
    println!("\n{}", style.bold("htmltoapk configuration"));
    let state = if exists { "" } else { " (defaults, file not created yet)" };
    println!("{}\n", style.dim(&format!("{}{state}", path.display())));

    let entries = config.entries();
    let width = entries.iter().map(|(key, _)| key.len()).max().unwrap_or(0);
    for (key, value) in entries {
        let shown = if value.is_empty() {
            style.dim("(unset)")
        } else {
            value
        };
        println!("  {:<width$}  {shown}", style.blue(key), width = width);
    }
    println!(
        "\n{}\n",
        style.dim("change a value with: htmltoapk config set <key> <value>")
    );
}

/// Print a doctor report.
pub fn print_doctor(report: &Report) {
    let style = Style::detect();
    println!("\n{}\n", style.bold("Environment check"));
    let width = report
        .checks
        .iter()
        .map(|check| check.name.len())
        .max()
        .unwrap_or(0);
    for check in &report.checks {
        let badge = match check.status {
            Status::Ok => style.green(" OK "),
            Status::Warn => style.yellow("WARN"),
            Status::Fail => style.red("FAIL"),
        };
        println!(
            "  [{badge}] {:<width$}  {}",
            check.name,
            check.detail,
            width = width
        );
        if let Some(hint) = &check.hint {
            for line in hint.lines() {
                println!("          {}", style.dim(line));
            }
        }
    }

    println!();
    if report.is_ok() {
        let warnings = report.warnings();
        if warnings == 0 {
            println!("{} everything is ready to build APKs.\n", style.green("ready:"));
        } else {
            println!(
                "{} builds should work, {warnings} warning(s) above.\n",
                style.yellow("ready:")
            );
        }
    } else {
        println!(
            "{} fix the FAIL rows above, then run `htmltoapk doctor` again.\n",
            style.red("blocked:")
        );
    }
}
