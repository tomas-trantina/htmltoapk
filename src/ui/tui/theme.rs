//! Colour palette and small styling helpers for the TUI.

use ratatui::style::{Color, Modifier, Style};

/// Accent colour used for focus, selection and progress.
pub const ACCENT: Color = Color::Rgb(122, 162, 247);
/// Secondary accent for headings.
pub const ACCENT_SOFT: Color = Color::Rgb(158, 206, 106);
pub const WARN: Color = Color::Rgb(224, 175, 104);
pub const ERROR: Color = Color::Rgb(247, 118, 142);
pub const MUTED: Color = Color::Rgb(120, 130, 156);
pub const TEXT: Color = Color::Rgb(213, 220, 240);
pub const PANEL_BORDER: Color = Color::Rgb(60, 68, 96);

pub fn title() -> Style {
    Style::default()
        .fg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

pub fn heading() -> Style {
    Style::default()
        .fg(ACCENT_SOFT)
        .add_modifier(Modifier::BOLD)
}

pub fn body() -> Style {
    Style::default().fg(TEXT)
}

pub fn muted() -> Style {
    Style::default().fg(MUTED)
}

pub fn selected() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

pub fn focused_border() -> Style {
    Style::default().fg(ACCENT)
}

pub fn border() -> Style {
    Style::default().fg(PANEL_BORDER)
}

pub fn warn() -> Style {
    Style::default().fg(WARN)
}

pub fn error() -> Style {
    Style::default().fg(ERROR)
}

pub fn success() -> Style {
    Style::default()
        .fg(ACCENT_SOFT)
        .add_modifier(Modifier::BOLD)
}
