//! TUI state machine.
//!
//! The application state never touches the terminal: `draw.rs` renders it and
//! `mod.rs` feeds it key events. Long running work (a build) happens in a
//! worker thread that talks back through a channel, so the interface stays
//! responsive and the very same `core::build::run` pipeline is reused.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::core::assets::SourceImage;
use crate::core::build::{self, BuildRequest, STAGES};
use crate::core::clean::{self, CleanReport, CleanTargets};
use crate::core::config::{self, BuildType, Config};
use crate::core::doctor::{self, Report};
use crate::core::input::WebInput;
use crate::core::report::{Level, Reporter};
use crate::core::{fsx, paths, zipper};

/// Visible screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Menu,
    Build,
    Config,
    Doctor,
    Zip,
    Clean,
    Help,
}

/// Main menu entries.
pub const MENU: [(&str, &str); 7] = [
    ("Build APK", "Package an HTML file or a web directory"),
    ("Configuration", "Edit global defaults"),
    ("Doctor", "Check node, npm, npx, java and the Android SDK"),
    ("Export ZIP", "Archive a project directory"),
    ("Clean", "Remove generated workspaces and logs"),
    ("Help", "Keys, commands and configuration keys"),
    ("Quit", "Leave htmltoapk"),
];

/// A single editable text field.
#[derive(Debug, Clone)]
pub struct Field {
    pub label: String,
    pub value: String,
    pub hint: String,
}

impl Field {
    fn new(label: &str, value: impl Into<String>, hint: &str) -> Self {
        Field {
            label: label.to_string(),
            value: value.into(),
            hint: hint.to_string(),
        }
    }
}

/// A vertical list of text fields with a cursor.
#[derive(Debug, Clone, Default)]
pub struct Form {
    pub fields: Vec<Field>,
    pub cursor: usize,
}

impl Form {
    fn next(&mut self) {
        if !self.fields.is_empty() {
            self.cursor = (self.cursor + 1) % self.fields.len();
        }
    }

    fn previous(&mut self) {
        if !self.fields.is_empty() {
            self.cursor = (self.cursor + self.fields.len() - 1) % self.fields.len();
        }
    }

    fn current_mut(&mut self) -> Option<&mut Field> {
        self.fields.get_mut(self.cursor)
    }

    pub fn value(&self, index: usize) -> String {
        self.fields
            .get(index)
            .map(|field| field.value.trim().to_string())
            .unwrap_or_default()
    }

    fn push_char(&mut self, ch: char) {
        if let Some(field) = self.current_mut() {
            field.value.push(ch);
        }
    }

    fn backspace(&mut self) {
        if let Some(field) = self.current_mut() {
            field.value.pop();
        }
    }

    fn clear(&mut self) {
        if let Some(field) = self.current_mut() {
            field.value.clear();
        }
    }
}

/// Progress messages sent from the build thread.
#[derive(Debug)]
pub enum BuildEvent {
    Stage {
        index: usize,
        total: usize,
        label: String,
    },
    Log(Level, String),
    Finished(std::result::Result<FinishedBuild, String>),
}

/// Successful build summary shown in the TUI.
#[derive(Debug, Clone)]
pub struct FinishedBuild {
    pub apk: PathBuf,
    pub size: String,
    pub seconds: u64,
    pub log: PathBuf,
}

/// Reporter that forwards progress to the UI thread.
struct ChannelReporter {
    tx: Sender<BuildEvent>,
}

impl Reporter for ChannelReporter {
    fn stage(&mut self, index: usize, total: usize, label: &str) {
        let _ = self.tx.send(BuildEvent::Stage {
            index,
            total,
            label: label.to_string(),
        });
    }

    fn log(&mut self, level: Level, message: &str) {
        let _ = self.tx.send(BuildEvent::Log(level, message.to_string()));
    }
}

/// Live build state.
#[derive(Debug, Default)]
pub struct BuildState {
    pub running: bool,
    pub stage: usize,
    pub total: usize,
    pub label: String,
    pub lines: Vec<(Level, String)>,
    pub outcome: Option<std::result::Result<FinishedBuild, String>>,
    rx: Option<Receiver<BuildEvent>>,
}

impl BuildState {
    pub fn ratio(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.stage as f64 / self.total as f64).clamp(0.0, 1.0)
    }
}

/// Configuration editor state.
#[derive(Debug, Default)]
pub struct ConfigEditor {
    pub cursor: usize,
    pub editing: bool,
    pub buffer: String,
}

/// Status line severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Success,
    Warn,
    Error,
}

/// Whole application state.
pub struct App {
    pub screen: Screen,
    pub menu: usize,
    pub should_quit: bool,
    pub status: Option<(StatusKind, String)>,
    pub config: Config,
    pub config_path: PathBuf,
    pub config_dirty: bool,
    pub build_form: Form,
    pub zip_form: Form,
    pub editor: ConfigEditor,
    pub doctor: Option<Report>,
    pub clean: Option<CleanReport>,
    pub build: BuildState,
}

impl App {
    /// Load configuration (falling back to defaults) and prime every form.
    pub fn new() -> Self {
        let config_path = Config::path().unwrap_or_else(|_| PathBuf::from("config.toml"));
        let (config, status) = match Config::load() {
            Ok(config) => (config, None),
            Err(error) => (
                Config::default(),
                Some((
                    StatusKind::Error,
                    format!("{error} (using built-in defaults)"),
                )),
            ),
        };

        let build_form = Form {
            fields: vec![
                Field::new("Input", "", "Path to an .html file or a web directory"),
                Field::new("App name", "", "Leave empty to derive it from the input"),
                Field::new("App id", "", "Leave empty to use appIdPrefix + app name"),
                Field::new("Output APK", "", "Leave empty for <outputDir>/<app>-<variant>.apk"),
                Field::new(
                    "Build type",
                    config.build_type.as_str(),
                    "debug or release",
                ),
                Field::new("Icon", "", "Optional source icon (png/jpg/svg)"),
                Field::new("Splash", "", "Optional source splash image"),
            ],
            cursor: 0,
        };

        let zip_form = Form {
            fields: vec![
                Field::new("Source", ".", "Directory or file to archive"),
                Field::new("Destination", "", "Leave empty for <source>.zip"),
            ],
            cursor: 0,
        };

        App {
            screen: Screen::Menu,
            menu: 0,
            should_quit: false,
            status,
            config,
            config_path,
            config_dirty: false,
            build_form,
            zip_form,
            editor: ConfigEditor::default(),
            doctor: None,
            clean: None,
            build: BuildState::default(),
        }
    }

    fn info(&mut self, message: impl Into<String>) {
        self.status = Some((StatusKind::Info, message.into()));
    }

    fn success(&mut self, message: impl Into<String>) {
        self.status = Some((StatusKind::Success, message.into()));
    }

    fn warn(&mut self, message: impl Into<String>) {
        self.status = Some((StatusKind::Warn, message.into()));
    }

    fn error(&mut self, message: impl Into<String>) {
        self.status = Some((StatusKind::Error, message.into()));
    }

    /// Keybinding hints rendered in the footer.
    pub fn footer(&self) -> &'static str {
        match self.screen {
            Screen::Menu => "Up/Down move   Enter select   q quit",
            Screen::Build => {
                if self.build.running {
                    "building...   Esc back when finished"
                } else {
                    "Tab/Up/Down field   type to edit   Ctrl+U clear   Enter build   Esc back"
                }
            }
            Screen::Config => {
                if self.editor.editing {
                    "type to edit   Enter apply   Esc cancel"
                } else {
                    "Up/Down key   Enter edit   s save   r reload   Esc back"
                }
            }
            Screen::Doctor => "r re-run   Esc back",
            Screen::Zip => "Tab field   Enter create archive   Esc back",
            Screen::Clean => "r rescan   d delete   Esc back",
            Screen::Help => "Esc back",
        }
    }

    // ----------------------------------------------------------------- events

    /// Drain progress from the build thread. Called on every tick.
    pub fn poll(&mut self) {
        let mut finished = false;
        if let Some(rx) = &self.build.rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    BuildEvent::Stage {
                        index,
                        total,
                        label,
                    } => {
                        self.build.stage = index;
                        self.build.total = total;
                        self.build.label = label;
                    }
                    BuildEvent::Log(level, message) => {
                        self.build.lines.push((level, message));
                        let overflow = self.build.lines.len().saturating_sub(400);
                        if overflow > 0 {
                            self.build.lines.drain(0..overflow);
                        }
                    }
                    BuildEvent::Finished(result) => {
                        self.build.outcome = Some(result);
                        self.build.running = false;
                        finished = true;
                    }
                }
            }
        }
        if finished {
            self.build.rx = None;
            match &self.build.outcome {
                Some(Ok(done)) => {
                    self.build.stage = self.build.total;
                    let message = format!("APK ready: {} ({})", done.apk.display(), done.size);
                    self.success(message);
                }
                Some(Err(message)) => {
                    let message = message.clone();
                    self.error(message);
                }
                None => {}
            }
        }
    }

    /// Handle a printable character.
    pub fn on_char(&mut self, ch: char) {
        match self.screen {
            Screen::Menu => match ch {
                'q' => self.should_quit = true,
                'b' => self.open(Screen::Build),
                'c' => self.open(Screen::Config),
                'd' => self.open(Screen::Doctor),
                'z' => self.open(Screen::Zip),
                'h' | '?' => self.open(Screen::Help),
                _ => {}
            },
            Screen::Build => {
                if !self.build.running {
                    self.build_form.push_char(ch);
                }
            }
            Screen::Zip => self.zip_form.push_char(ch),
            Screen::Config => {
                if self.editor.editing {
                    self.editor.buffer.push(ch);
                } else {
                    match ch {
                        's' => self.save_config(),
                        'r' => self.reload_config(),
                        _ => {}
                    }
                }
            }
            Screen::Doctor => {
                if ch == 'r' {
                    self.run_doctor();
                }
            }
            Screen::Clean => match ch {
                'r' => self.scan_clean(),
                'd' => self.run_clean(),
                _ => {}
            },
            Screen::Help => {}
        }
    }

    pub fn on_backspace(&mut self) {
        match self.screen {
            Screen::Build if !self.build.running => self.build_form.backspace(),
            Screen::Zip => self.zip_form.backspace(),
            Screen::Config if self.editor.editing => {
                self.editor.buffer.pop();
            }
            _ => {}
        }
    }

    pub fn on_clear_field(&mut self) {
        match self.screen {
            Screen::Build if !self.build.running => self.build_form.clear(),
            Screen::Zip => self.zip_form.clear(),
            Screen::Config if self.editor.editing => self.editor.buffer.clear(),
            _ => {}
        }
    }

    pub fn on_up(&mut self) {
        match self.screen {
            Screen::Menu => {
                self.menu = (self.menu + MENU.len() - 1) % MENU.len();
            }
            Screen::Build => self.build_form.previous(),
            Screen::Zip => self.zip_form.previous(),
            Screen::Config if !self.editor.editing => {
                self.editor.cursor =
                    (self.editor.cursor + config::KEYS.len() - 1) % config::KEYS.len();
            }
            _ => {}
        }
    }

    pub fn on_down(&mut self) {
        match self.screen {
            Screen::Menu => {
                self.menu = (self.menu + 1) % MENU.len();
            }
            Screen::Build => self.build_form.next(),
            Screen::Zip => self.zip_form.next(),
            Screen::Config if !self.editor.editing => {
                self.editor.cursor = (self.editor.cursor + 1) % config::KEYS.len();
            }
            _ => {}
        }
    }

    pub fn on_escape(&mut self) {
        match self.screen {
            Screen::Menu => self.should_quit = true,
            Screen::Config if self.editor.editing => {
                self.editor.editing = false;
                self.editor.buffer.clear();
            }
            Screen::Build if self.build.running => {
                self.warn("A build is running, please wait for it to finish");
            }
            _ => {
                self.screen = Screen::Menu;
                self.status = None;
            }
        }
    }

    pub fn on_enter(&mut self) {
        match self.screen {
            Screen::Menu => self.activate_menu(),
            Screen::Build => self.start_build(),
            Screen::Zip => self.create_archive(),
            Screen::Config => self.toggle_editor(),
            Screen::Doctor => self.run_doctor(),
            Screen::Clean => self.run_clean(),
            Screen::Help => self.screen = Screen::Menu,
        }
    }

    fn open(&mut self, screen: Screen) {
        self.screen = screen;
        self.status = None;
        match screen {
            Screen::Doctor => self.run_doctor(),
            Screen::Clean => self.scan_clean(),
            _ => {}
        }
    }

    fn activate_menu(&mut self) {
        match self.menu {
            0 => self.open(Screen::Build),
            1 => self.open(Screen::Config),
            2 => self.open(Screen::Doctor),
            3 => self.open(Screen::Zip),
            4 => self.open(Screen::Clean),
            5 => self.open(Screen::Help),
            _ => self.should_quit = true,
        }
    }

    // ------------------------------------------------------------- operations

    fn run_doctor(&mut self) {
        let report = doctor::run();
        if report.is_ok() {
            if report.warnings() == 0 {
                self.success("Environment is ready to build APKs");
            } else {
                self.warn(format!(
                    "Builds should work, {} warning(s) found",
                    report.warnings()
                ));
            }
        } else {
            self.error("Required tools are missing, see the FAIL rows");
        }
        self.doctor = Some(report);
    }

    fn scan_clean(&mut self) {
        match clean::scan(&self.config, CleanTargets::default()) {
            Ok(report) => {
                if report.is_empty() {
                    self.info("Nothing to clean");
                } else {
                    self.info(format!(
                        "{} item(s), {} reclaimable",
                        report.candidates.len(),
                        fsx::human_size(report.total_bytes())
                    ));
                }
                self.clean = Some(report);
            }
            Err(error) => self.error(error.to_string()),
        }
    }

    fn run_clean(&mut self) {
        let mut report = match self.clean.take() {
            Some(report) => report,
            None => return,
        };
        if report.is_empty() {
            self.info("Nothing to clean");
            self.clean = Some(report);
            return;
        }
        let freed = fsx::human_size(report.total_bytes());
        match clean::remove(&mut report) {
            Ok(()) => {
                self.success(format!("Removed {} item(s), freed {freed}", report.removed.len()));
                self.scan_clean();
            }
            Err(error) => {
                self.error(error.to_string());
                self.clean = Some(report);
            }
        }
    }

    fn toggle_editor(&mut self) {
        let key = config::KEYS[self.editor.cursor];
        if self.editor.editing {
            let value = self.editor.buffer.clone();
            match self.config.set(key, &value) {
                Ok(()) => {
                    self.config_dirty = true;
                    self.editor.editing = false;
                    self.editor.buffer.clear();
                    self.info(format!("{key} updated, press `s` to save"));
                }
                Err(error) => self.error(error.to_string()),
            }
        } else {
            self.editor.buffer = self.config.get(key).unwrap_or_default();
            self.editor.editing = true;
            self.info(config::describe(key));
        }
    }

    fn save_config(&mut self) {
        match self.config.save() {
            Ok(path) => {
                self.config_dirty = false;
                self.success(format!("Configuration saved to {}", path.display()));
            }
            Err(error) => self.error(error.to_string()),
        }
    }

    fn reload_config(&mut self) {
        match Config::load() {
            Ok(config) => {
                self.config = config;
                self.config_dirty = false;
                self.info("Configuration reloaded from disk");
            }
            Err(error) => self.error(error.to_string()),
        }
    }

    fn create_archive(&mut self) {
        let source = self.zip_form.value(0);
        let source = if source.is_empty() { ".".to_string() } else { source };
        let source = paths::absolute(&PathBuf::from(source));
        let destination = self.zip_form.value(1);
        let destination = if destination.is_empty() {
            paths::absolute(&zipper::default_destination(&source))
        } else {
            paths::absolute(&PathBuf::from(destination))
        };
        match zipper::zip_dir(&source, &destination) {
            Ok(path) => {
                let size = fsx::human_size(fsx::size_of(&path));
                self.success(format!("Archive created: {} ({size})", path.display()));
            }
            Err(error) => self.error(error.to_string()),
        }
    }

    fn start_build(&mut self) {
        if self.build.running {
            return;
        }
        match self.prepare_request() {
            Ok(request) => {
                self.build = BuildState {
                    running: true,
                    stage: 0,
                    total: STAGES.len(),
                    label: STAGES[0].to_string(),
                    lines: Vec::new(),
                    outcome: None,
                    rx: None,
                };
                let (tx, rx) = mpsc::channel();
                self.build.rx = Some(rx);
                self.info(format!("Building {}", request.app_name));
                thread::spawn(move || {
                    let mut reporter = ChannelReporter { tx: tx.clone() };
                    let result = build::run(&request, &mut reporter);
                    let payload = match result {
                        Ok(outcome) => Ok(FinishedBuild {
                            apk: outcome.apk,
                            size: fsx::human_size(outcome.apk_size),
                            seconds: outcome.seconds,
                            log: outcome.log,
                        }),
                        Err(error) => {
                            let mut message = error.to_string();
                            if let Some(hint) = error.hint() {
                                if let Some(first) = hint.lines().next() {
                                    message.push_str(" — ");
                                    message.push_str(first);
                                }
                            }
                            Err(message)
                        }
                    };
                    let _ = tx.send(BuildEvent::Finished(payload));
                });
            }
            Err(message) => self.error(message),
        }
    }

    /// Translate the build form into a validated [`BuildRequest`].
    fn prepare_request(&self) -> std::result::Result<BuildRequest, String> {
        let input_value = self.build_form.value(0);
        if input_value.is_empty() {
            return Err("Please enter the input HTML file or web directory".to_string());
        }
        let input = WebInput::resolve(&paths::expand_tilde(&PathBuf::from(&input_value)))
            .map_err(|error| error.to_string())?;

        let derived = input.suggested_app_name();
        let name_field = self.build_form.value(1);
        let app_name = self.config.resolve_app_name(
            if name_field.is_empty() {
                None
            } else {
                Some(name_field.as_str())
            },
            Some(&derived),
        );

        let id_field = self.build_form.value(2);
        let app_id = self
            .config
            .resolve_app_id(
                if id_field.is_empty() {
                    None
                } else {
                    Some(id_field.as_str())
                },
                &app_name,
            )
            .map_err(|error| error.to_string())?;

        let variant_field = self.build_form.value(4);
        let build_type = if variant_field.is_empty() {
            self.config.build_type
        } else {
            BuildType::parse(&variant_field).map_err(|error| error.to_string())?
        };

        let output_field = self.build_form.value(3);
        let output = if output_field.is_empty() {
            build::default_output(&self.config, &app_name, build_type)
        } else {
            paths::expand_tilde(&PathBuf::from(output_field))
        };

        let mut request =
            BuildRequest::new(&self.config, input, output, app_name, app_id, build_type);

        let icon_field = self.build_form.value(5);
        let icon = if icon_field.is_empty() {
            self.config.assets.icon.clone()
        } else {
            Some(PathBuf::from(icon_field))
        };
        if let Some(icon) = icon {
            request.icon = Some(
                SourceImage::resolve(&paths::expand_tilde(&icon), "icon")
                    .map_err(|error| error.to_string())?,
            );
        }

        let splash_field = self.build_form.value(6);
        let splash = if splash_field.is_empty() {
            self.config.assets.splash.clone()
        } else {
            Some(PathBuf::from(splash_field))
        };
        if let Some(splash) = splash {
            request.splash = Some(
                SourceImage::resolve(&paths::expand_tilde(&splash), "splash")
                    .map_err(|error| error.to_string())?,
            );
        }

        request.validate().map_err(|error| error.to_string())?;
        Ok(request)
    }
}

impl Default for App {
    fn default() -> Self {
        App::new()
    }
}
