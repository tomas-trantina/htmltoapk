//! Terminal user interface (ratatui + crossterm).
//!
//! Started when `htmltoapk` runs without arguments. Every action available in
//! the TUI maps to the same core functions used by the CLI.

pub mod app;
pub mod draw;
pub mod theme;

use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::error::{Error, Result};
use app::App;

/// Run the interactive interface until the user quits.
pub fn run() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let mut app = App::new();

    while !app.should_quit {
        app.poll();
        terminal
            .draw(|frame| draw::draw(frame, &app))
            .map_err(|err| Error::io("could not render the interface", err))?;

        let pending = event::poll(Duration::from_millis(120))
            .map_err(|err| Error::io("could not read terminal events", err))?;
        if !pending {
            continue;
        }
        let next = event::read().map_err(|err| Error::io("could not read a terminal event", err))?;
        if let Event::Key(key) = next {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            match key.code {
                KeyCode::Char('c') if ctrl => app.should_quit = true,
                KeyCode::Char('u') if ctrl => app.on_clear_field(),
                KeyCode::Char(ch) => app.on_char(ch),
                KeyCode::Backspace => app.on_backspace(),
                KeyCode::Enter => app.on_enter(),
                KeyCode::Esc => app.on_escape(),
                KeyCode::Up => app.on_up(),
                KeyCode::Down | KeyCode::Tab => app.on_down(),
                KeyCode::BackTab => app.on_up(),
                _ => {}
            }
        }
    }

    Ok(())
}
