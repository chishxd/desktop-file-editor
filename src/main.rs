mod utility;

use std::path::PathBuf;

use crossterm::event::{self, KeyCode};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Stylize},
    text::{Line, Span},
    widgets::{List, ListState, Paragraph},
};

use crate::utility::{load_files, parse_file};

struct RawFile {
    name: String,
    path: PathBuf,
}

#[derive(Debug)]
struct ParsedDesktopFile {
    name: String,
    exec: String,
    icon: String,
}

struct EditState {
    key: String,
    value: String,
    buf: String,
}

enum AppMode {
    Browse,
    Edit {
        file_idx: usize,
        field: String,
        editing_state: EditState,
    },
}
struct App {
    items: Vec<RawFile>,
    idx: usize,
    parsed_file: Option<ParsedDesktopFile>,
    mode: AppMode,
}

fn main() -> std::io::Result<()> {
    let mut list_state = ListState::default().with_selected(Some(0));
    let app = App::new()?;
    app.run(&mut list_state)
}

impl App {
    fn new() -> std::io::Result<Self> {
        let mut app = Self {
            items: load_files()?,
            idx: 0,
            parsed_file: None,
            mode: AppMode::Browse,
        };

        app.parse_current();
        Ok(app)
    }

    fn run(mut self, list_state: &mut ListState) -> std::io::Result<()> {
        ratatui::run(|terminal| {
            loop {
                terminal.draw(|frame| self.render(frame, list_state))?;

                if let Some(key) = event::read()?.as_key_press_event() {
                    match &mut self.mode {
                        AppMode::Browse => match key.code {
                            KeyCode::Char('j') | KeyCode::Down => {
                                if !self.items.is_empty() {
                                    self.idx = (self.idx + 1).min(self.items.len() - 1);
                                    self.parse_current();
                                }
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                self.idx = self.idx.saturating_sub(1);
                                self.parse_current();
                            }
                            KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                            KeyCode::Enter => {
                                if let Some(parsed_file) = &self.parsed_file {
                                    let edit_state = EditState {
                                        key: "Name".to_string(),
                                        value: parsed_file.name.clone(),
                                        buf: parsed_file.name.clone(),
                                    };

                                    self.mode = AppMode::Edit {
                                        file_idx: self.idx,
                                        field: "Name".to_string(),
                                        editing_state: edit_state,
                                    }
                                }
                            }
                            _ => {}
                        },
                        AppMode::Edit { editing_state, .. } => match key.code {
                            KeyCode::Char(c) => {
                                editing_state.buf.push(c);
                            }
                            KeyCode::Backspace => {
                                editing_state.buf.pop();
                            }
                            KeyCode::Esc => self.mode = AppMode::Browse,
                            _ => {}
                        },
                    }
                }
            }
        })
    }

    fn parse_current(&mut self) {
        self.parsed_file = self
            .items
            .get(self.idx)
            .and_then(|f| parse_file(&f.path).ok());
    }
    fn render(&self, frame: &mut Frame, list_state: &mut ListState) {
        let vertical = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).spacing(1);
        let [title_area, body_area] = frame.area().layout(&vertical);

        let cols = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .spacing(1)
            .split(body_area);
        let left = cols[0];
        let right = cols[1];

        let title = Line::from_iter([
            Span::from("Desktop File Editor").bold(),
            Span::from("  q: quit  j/k: move  enter: edit"),
        ]);
        frame.render_widget(title.centered(), title_area);

        let names: Vec<String> = self.items.iter().map(|f| f.name.clone()).collect();
        list_state.select(Some(self.idx));
        self.render_list(names, frame, left, list_state);

        match self.mode {
            AppMode::Browse => self.render_desktop_editor(frame, right),
            AppMode::Edit { .. } => self.render_editor(frame, right),
        }
    }

    fn render_list(
        &self,
        items: Vec<String>,
        frame: &mut Frame,
        area: Rect,
        list_state: &mut ListState,
    ) {
        if !items.is_empty() {
            let list = List::new(items)
                .style(Color::White)
                .highlight_style(Modifier::REVERSED)
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, area, list_state);
        } else {
            frame.render_widget("No files found", frame.area());
        }
    }

    fn render_desktop_editor(&self, frame: &mut Frame, area: Rect) {
        let text: Vec<Line<'static>> = match self.parsed_file.as_ref() {
            Some(file) => vec![
                format!("Name: {}", file.name).into(),
                format!("Exec: {}", file.exec).into(),
                format!("Icon: {}", file.icon).into(),
            ],
            None => vec!["No file selected".into()],
        };

        let paragraph = Paragraph::new(text).block(
            ratatui::widgets::Block::default()
                .title("Details")
                .borders(ratatui::widgets::Borders::ALL),
        );
        frame.render_widget(paragraph, area);
    }
    fn render_editor(&self, frame: &mut Frame, area: Rect) {
        let lines: Vec<Line<'static>> = match self.parsed_file.as_ref() {
            Some(file) => vec![
                format!("Editing: {}", file.name).into(),
                "Type text, enter to save, esc to cancel".into(),
            ],
            None => vec!["Nothing to edit".into()],
        };

        let paragraph = Paragraph::new(lines).block(
            ratatui::widgets::Block::default()
                .title("Editor")
                .borders(ratatui::widgets::Borders::ALL),
        );

        frame.render_widget(paragraph, area);
    }
}
