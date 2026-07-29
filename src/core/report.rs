//! Progress reporting abstraction shared by every front-end.
//!
//! The core build pipeline emits staged progress through this trait, so the CLI
//! can stream it to stdout while the TUI renders it into a gauge + log pane
//! without duplicating any build logic.

/// Severity of a single log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Success,
}

/// Receives progress from long-running core operations.
pub trait Reporter: Send {
    /// A new pipeline stage started (`index` is 1-based).
    fn stage(&mut self, index: usize, total: usize, label: &str);

    /// A single log line.
    fn log(&mut self, level: Level, message: &str);

    fn info(&mut self, message: &str) {
        self.log(Level::Info, message);
    }

    fn warn(&mut self, message: &str) {
        self.log(Level::Warn, message);
    }

    fn success(&mut self, message: &str) {
        self.log(Level::Success, message);
    }
}

/// Reporter that throws everything away (useful for tests and scripting).
pub struct Silent;

impl Reporter for Silent {
    fn stage(&mut self, _index: usize, _total: usize, _label: &str) {}
    fn log(&mut self, _level: Level, _message: &str) {}
}
