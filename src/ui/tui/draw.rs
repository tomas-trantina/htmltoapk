//! Rendering. Pure functions of [`App`] state.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::core::config;
use crate::core::doctor::Status;
use crate::core::fsx;
use crate::core::report::Level;
use crate::ui::tui::app::{App, Form, Screen, StatusKind, MENU};
use crate::ui::tui::theme;

/// Draw the whole interface.
pub fn draw(frame: &mut Frame, app: &App) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(frame.area());

    header(frame, areas[0], app);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(30)])
        .split(areas[1]);

    menu(frame, columns[0], app);

    match app.screen {
        Screen::Menu => overview(frame, columns[1], app),
        Screen::Build => build_screen(frame, columns[1], app),
        Screen::Config => config_screen(frame, columns[1], app),
        Screen::Doctor => doctor_screen(frame, columns[1], app),
        Screen::Zip => form_screen(
            frame,
            columns[1],
            "Export ZIP",
            &app.zip_form,
            true,
            "Archives a directory or file, skipping node_modules, .git and build output.",
        ),
        Screen::Clean => clean_screen(frame, columns[1], app),
        Screen::Help => help_screen(frame, columns[1]),
    }

    footer(frame, areas[2], app);
}

fn panel(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border())
        .title(Span::styled(format!(" {title} "), theme::heading()))
}

fn header(frame: &mut Frame, area: Rect, app: &App) {
    let dirty = if app.config_dirty { "  (unsaved changes)" } else { "" };
    let text = Text::from(vec![Line::from(vec![
        Span::styled("htmltoapk", theme::title()),
        Span::styled(
            format!("  v{}", env!("CARGO_PKG_VERSION")),
            theme::muted(),
        ),
        Span::styled("   HTML → Android APK via Capacitor", theme::body()),
        Span::styled(
            format!("   {}{dirty}", app.config_path.display()),
            theme::muted(),
        ),
    ])]);
    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme::focused_border()),
        ),
        area,
    );
}

fn menu(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = MENU
        .iter()
        .enumerate()
        .map(|(index, (label, _))| {
            let selected = app.screen == Screen::Menu && index == app.menu;
            let style = if selected {
                theme::selected()
            } else {
                theme::body()
            };
            ListItem::new(Line::from(Span::styled(format!(" {label}"), style)))
        })
        .collect();
    frame.render_widget(List::new(items).block(panel("Menu")), area);
}

fn overview(frame: &mut Frame, area: Rect, app: &App) {
    let config = &app.config;
    let mut lines = vec![
        Line::from(Span::styled(
            MENU[app.menu].1.to_string(),
            theme::body(),
        )),
        Line::from(""),
        Line::from(Span::styled("Current defaults", theme::heading())),
    ];
    for (key, value) in [
        ("appIdPrefix", config.app_id_prefix.clone()),
        ("appName", config.app_name.clone()),
        ("buildType", config.build_type.to_string()),
        ("signing", config.signing.to_string()),
        ("workspace", config.workspace_root().display().to_string()),
        ("outputDir", config.output_root().display().to_string()),
    ] {
        lines.push(Line::from(vec![
            Span::styled(format!("  {key:<12}"), theme::muted()),
            Span::styled(value, theme::body()),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Shortcuts", theme::heading())));
    lines.push(Line::from(Span::styled(
        "  b build    c config    d doctor    z zip    h help    q quit",
        theme::muted(),
    )));
    frame.render_widget(
        Paragraph::new(lines).block(panel("Overview")).wrap(Wrap { trim: true }),
        area,
    );
}

fn form_lines(form: &Form, editable: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (index, field) in form.fields.iter().enumerate() {
        let active = editable && index == form.cursor;
        let marker = if active { "❯ " } else { "  " };
        let value = if field.value.is_empty() {
            "(default)".to_string()
        } else {
            field.value.clone()
        };
        let value_style = if field.value.is_empty() {
            theme::muted()
        } else {
            theme::body()
        };
        let label_style: Style = if active {
            theme::focused_border()
        } else {
            theme::muted()
        };
        let cursor = if active { "█" } else { "" };
        lines.push(Line::from(vec![
            Span::styled(marker.to_string(), theme::focused_border()),
            Span::styled(format!("{:<12}", field.label), label_style),
            Span::styled(format!("{value}{cursor}"), value_style),
        ]));
        if active {
            lines.push(Line::from(Span::styled(
                format!("              {}", field.hint),
                theme::muted(),
            )));
        }
    }
    lines
}

fn form_screen(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    form: &Form,
    editable: bool,
    note: &str,
) {
    let mut lines = form_lines(form, editable);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(note.to_string(), theme::muted())));
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel(title))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn build_screen(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(9),
            Constraint::Length(3),
            Constraint::Min(6),
        ])
        .split(area);

    form_screen(
        frame,
        rows[0],
        "Build APK",
        &app.build_form,
        !app.build.running,
        "Empty fields fall back to the configuration defaults.",
    );

    let label = if app.build.total == 0 {
        "idle".to_string()
    } else {
        format!(
            "{}/{}  {}",
            app.build.stage, app.build.total, app.build.label
        )
    };
    frame.render_widget(
        Gauge::default()
            .block(panel("Progress"))
            .gauge_style(Style::default().fg(theme::ACCENT))
            .ratio(app.build.ratio())
            .label(label),
        rows[1],
    );

    let mut lines: Vec<Line> = app
        .build
        .lines
        .iter()
        .map(|(level, message)| {
            let (marker, style) = match level {
                Level::Info => ("·", theme::body()),
                Level::Warn => ("!", theme::warn()),
                Level::Success => ("✓", theme::success()),
            };
            Line::from(vec![
                Span::styled(format!(" {marker} "), style),
                Span::styled(message.clone(), theme::body()),
            ])
        })
        .collect();

    match &app.build.outcome {
        Some(Ok(done)) => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!(
                    " APK: {}  ({}, {}s)",
                    done.apk.display(),
                    done.size,
                    done.seconds
                ),
                theme::success(),
            )));
            lines.push(Line::from(Span::styled(
                format!(" log: {}", done.log.display()),
                theme::muted(),
            )));
        }
        Some(Err(message)) => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!(" {message}"),
                theme::error(),
            )));
        }
        None => {}
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            " Press Enter to start the build.",
            theme::muted(),
        )));
    }

    let visible = rows[2].height.saturating_sub(2) as usize;
    let skip = lines.len().saturating_sub(visible.max(1));
    let lines: Vec<Line> = lines.into_iter().skip(skip).collect();

    frame.render_widget(
        Paragraph::new(lines)
            .block(panel("Build log"))
            .wrap(Wrap { trim: false }),
        rows[2],
    );
}

fn config_screen(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(5)])
        .split(area);

    let visible = rows[0].height.saturating_sub(2).max(1) as usize;
    let total = config::KEYS.len();
    let start = app
        .editor
        .cursor
        .saturating_sub(visible / 2)
        .min(total.saturating_sub(visible));

    let mut lines = Vec::new();
    for index in start..(start + visible).min(total) {
        let key = config::KEYS[index];
        let active = index == app.editor.cursor;
        let value = if active && app.editor.editing {
            format!("{}█", app.editor.buffer)
        } else {
            let value = app.config.get(key).unwrap_or_default();
            if value.is_empty() {
                "(unset)".to_string()
            } else {
                value
            }
        };
        let key_style = if active {
            theme::focused_border()
        } else {
            theme::muted()
        };
        lines.push(Line::from(vec![
            Span::styled(if active { "❯ " } else { "  " }, theme::focused_border()),
            Span::styled(format!("{key:<28}"), key_style),
            Span::styled(value, theme::body()),
        ]));
    }

    frame.render_widget(
        Paragraph::new(lines).block(panel("Configuration")),
        rows[0],
    );

    let key = config::KEYS[app.editor.cursor];
    let detail = vec![
        Line::from(vec![
            Span::styled("key   ", theme::muted()),
            Span::styled(key.to_string(), theme::body()),
        ]),
        Line::from(vec![
            Span::styled("about ", theme::muted()),
            Span::styled(config::describe(key).to_string(), theme::body()),
        ]),
        Line::from(Span::styled(
            "Enter edits the value, `s` writes the file, `r` reloads it.",
            theme::muted(),
        )),
    ];
    frame.render_widget(Paragraph::new(detail).block(panel("Details")).wrap(Wrap { trim: true }), rows[1]);
}

fn doctor_screen(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines = Vec::new();
    match &app.doctor {
        None => lines.push(Line::from(Span::styled(
            " Press Enter to run the checks.",
            theme::muted(),
        ))),
        Some(report) => {
            for check in &report.checks {
                let (badge, style) = match check.status {
                    Status::Ok => (" OK ", theme::success()),
                    Status::Warn => ("WARN", theme::warn()),
                    Status::Fail => ("FAIL", theme::error()),
                };
                lines.push(Line::from(vec![
                    Span::styled(format!(" [{badge}] "), style),
                    Span::styled(format!("{:<14}", check.name), theme::body()),
                    Span::styled(check.detail.clone(), theme::muted()),
                ]));
                if let Some(hint) = &check.hint {
                    for line in hint.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("        {line}"),
                            theme::muted(),
                        )));
                    }
                }
            }
        }
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel("Doctor"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn clean_screen(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines = Vec::new();
    match &app.clean {
        None => lines.push(Line::from(Span::styled(
            " Press `r` to scan for removable items.",
            theme::muted(),
        ))),
        Some(report) if report.is_empty() => lines.push(Line::from(Span::styled(
            " Nothing to clean.",
            theme::success(),
        ))),
        Some(report) => {
            for candidate in &report.candidates {
                lines.push(Line::from(vec![
                    Span::styled(format!(" {:<10}", candidate.label), theme::muted()),
                    Span::styled(candidate.path.display().to_string(), theme::body()),
                    Span::styled(
                        format!("  ({})", fsx::human_size(candidate.bytes)),
                        theme::muted(),
                    ),
                ]));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!(
                    " Total {} in {} item(s). Press `d` to delete.",
                    fsx::human_size(report.total_bytes()),
                    report.candidates.len()
                ),
                theme::warn(),
            )));
        }
    }
    frame.render_widget(
        Paragraph::new(lines).block(panel("Clean")).wrap(Wrap { trim: false }),
        area,
    );
}

fn help_screen(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled("Navigation", theme::heading())),
        Line::from(Span::styled(
            "  Up/Down or Tab move   Enter confirm   Esc back   q quit",
            theme::body(),
        )),
        Line::from(""),
        Line::from(Span::styled("Screens", theme::heading())),
        Line::from(Span::styled(
            "  Build APK      build from a single HTML file or a web directory",
            theme::body(),
        )),
        Line::from(Span::styled(
            "  Configuration  edit every default, save with `s`",
            theme::body(),
        )),
        Line::from(Span::styled(
            "  Doctor         verify node, npm, npx, java, Android SDK",
            theme::body(),
        )),
        Line::from(Span::styled(
            "  Export ZIP     archive a project directory",
            theme::body(),
        )),
        Line::from(Span::styled(
            "  Clean          remove generated workspaces and logs",
            theme::body(),
        )),
        Line::from(""),
        Line::from(Span::styled("Equivalent CLI commands", theme::heading())),
        Line::from(Span::styled(
            "  htmltoapk setup | doctor | config [get|set] | clean | zip",
            theme::body(),
        )),
        Line::from(Span::styled(
            "  htmltoapk make <input> <output.apk>",
            theme::body(),
        )),
        Line::from(Span::styled(
            "  htmltoapk make-dir <dir> <output.apk>",
            theme::body(),
        )),
        Line::from(""),
        Line::from(Span::styled("Requirements", theme::heading())),
        Line::from(Span::styled(
            "  Node.js 18+, npm 9+, npx, JDK 17, Android SDK (ANDROID_HOME)",
            theme::body(),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(panel("Help")).wrap(Wrap { trim: true }),
        area,
    );
}

fn footer(frame: &mut Frame, area: Rect, app: &App) {
    let (style, message) = match &app.status {
        Some((StatusKind::Success, text)) => (theme::success(), text.clone()),
        Some((StatusKind::Warn, text)) => (theme::warn(), text.clone()),
        Some((StatusKind::Error, text)) => (theme::error(), text.clone()),
        Some((StatusKind::Info, text)) => (theme::body(), text.clone()),
        None => (theme::muted(), app.footer().to_string()),
    };
    let lines = vec![Line::from(vec![
        Span::styled(format!(" {message}"), style),
    ])];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(theme::border())
                    .title(Span::styled(
                        format!(" {} ", app.footer()),
                        theme::muted(),
                    )),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}
