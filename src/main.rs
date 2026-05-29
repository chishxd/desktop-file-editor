mod utility;

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use crossterm::event::{self, KeyCode};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, List, ListState, Paragraph},
};
use ratatui_textarea::{CursorMove, Input, TextArea};

use crate::utility::{load_files, parse_file, spawn_update_desktop_database};

struct RawFile {
    name: String,
    path: PathBuf,
}

enum DbUpdateResult {
    Updated,
    MissingBinary,
    Failed(String),
}

#[derive(Debug, Clone)]
struct ParsedDesktopFile {
    name: String,
    exec: String,
    icon: String,
}

enum AppMode {
    Browse,
    Search {
        input: TextArea<'static>,
    },
    Edit {
        file_idx: usize,
        active_field: usize,
        textareas: Vec<TextArea<'static>>,
        search_input: Option<TextArea<'static>>,
    },
}
struct App {
    items: Vec<RawFile>,
    idx: usize,
    parsed_file: Option<ParsedDesktopFile>,
    mode: AppMode,
    status_message: Option<String>,
    status_style: Style,
    status_created: Option<std::time::Instant>,
    status_job: Option<std::sync::mpsc::Receiver<DbUpdateResult>>,
    displaying_indices: Vec<usize>,
}

fn main() -> std::io::Result<()> {
    let mut list_state = ListState::default().with_selected(Some(0));
    let app = App::new()?;
    app.run(&mut list_state)
}

impl App {
    // The constructor for App's State
    fn new() -> std::io::Result<Self> {
        let mut app = Self {
            items: load_files()?,
            idx: 0,
            parsed_file: None,
            mode: AppMode::Browse,
            status_created: None,
            status_message: None,
            status_style: Style::default(),
            status_job: None,
            displaying_indices: (0..0).collect(),
        };

        app.displaying_indices = (0..app.items.len()).collect();
        app.parse_current();
        Ok(app)
    }

    // This function updates the indices to be displayed based on query
    fn update_displaying_indices(&mut self, query: &str) {
        let q = query.to_lowercase();
        self.displaying_indices = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                if f.name.to_lowercase().contains(&q) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
    }

    fn reset_search_state(&mut self) {
        self.displaying_indices = (0..self.items.len()).collect();
        self.idx = 0;
        self.parse_current();
    }

    fn make_edit_mode(
        parsed_file: ParsedDesktopFile,
        file_idx: usize,
        search_input: Option<TextArea<'static>>,
    ) -> AppMode {
        let mut name = TextArea::from(vec![parsed_file.name]);
        let mut exec = TextArea::from(vec![parsed_file.exec]);
        let mut icon = TextArea::from(vec![parsed_file.icon]);

        for ta in [&mut name, &mut exec, &mut icon] {
            ta.set_cursor_line_style(Style::default());
            ta.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
            ta.move_cursor(CursorMove::End);
        }

        AppMode::Edit {
            file_idx,
            active_field: 0,
            textareas: vec![name, exec, icon],
            search_input,
        }
    }

    // The main run function
    fn run(mut self, list_state: &mut ListState) -> std::io::Result<()> {
        ratatui::run(|terminal| {
            loop {
                self.clear_status_msg();
                self.poll_status_job();
                terminal.draw(|frame| self.render(frame, list_state))?;

                if event::poll(Duration::from_millis(100))? {
                    if let Some(key) = event::read()?.as_key_press_event() {
                        match &mut self.mode {
                            AppMode::Browse => match key.code {
                                KeyCode::Esc => {
                                    self.reset_search_state();
                                }
                                KeyCode::Char('/') => {
                                    let mut input = TextArea::from(vec![String::new()]);
                                    input.set_cursor_line_style(Style::default());
                                    input.set_cursor_style(
                                        Style::default().add_modifier(Modifier::REVERSED),
                                    );
                                    input.move_cursor(CursorMove::End);

                                    self.mode = AppMode::Search { input };
                                }
                                KeyCode::Char('j') | KeyCode::Down => {
                                    if !self.items.is_empty() {
                                        self.idx =
                                            (self.idx + 1).min(self.displaying_indices.len() - 1);
                                        self.parse_current();
                                    }
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    self.idx = self.idx.saturating_sub(1);
                                    self.parse_current();
                                }
                                KeyCode::Char('q') => break Ok(()),
                                KeyCode::Enter => {
                                    if let Some(parsed_file) = self.parsed_file.clone() {
                                        self.mode =
                                            Self::make_edit_mode(parsed_file, self.idx, None);
                                    }
                                }
                                _ => {}
                            },
                            AppMode::Search { input } => match key.code {
                                KeyCode::Esc => {
                                    self.mode = AppMode::Browse;
                                    self.reset_search_state();
                                }
                                KeyCode::Char('j') | KeyCode::Down => {
                                    if !self.items.is_empty() {
                                        self.idx =
                                            (self.idx + 1).min(self.displaying_indices.len() - 1);
                                        self.parse_current();
                                    }
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    self.idx = self.idx.saturating_sub(1);
                                    self.parse_current();
                                }
                                KeyCode::Enter => {
                                    // Map filtered index back to real index and enter edit mode (search -> edit).
                                    if let Some(real_idx) =
                                        self.displaying_indices.get(self.idx).copied()
                                    {
                                        if let Some(parsed_file) = self.parsed_file.clone() {
                                            self.mode = Self::make_edit_mode(
                                                parsed_file,
                                                real_idx,
                                                Some(input.clone()),
                                            );
                                        }
                                    }
                                }
                                _ => {
                                    let ta_input: Input = key.into();
                                    input.input(ta_input);

                                    let query = input.lines().first().cloned().unwrap_or_default();
                                    self.update_displaying_indices(&query);
                                    self.idx = 0;
                                    self.parse_current();
                                }
                            },
                            AppMode::Edit {
                                file_idx,
                                active_field,
                                textareas,
                                search_input,
                            } => match key.code {
                                KeyCode::Tab | KeyCode::Down => {
                                    *active_field = (*active_field + 1) % textareas.len();
                                }
                                KeyCode::Up => {
                                    *active_field = active_field.saturating_sub(1);
                                }
                                KeyCode::Esc => {
                                    if let Some(search_input) = search_input.take() {
                                        self.mode = AppMode::Search {
                                            input: search_input,
                                        };
                                    } else {
                                        self.mode = AppMode::Browse;
                                    }
                                }
                                KeyCode::Enter => {
                                    let new_name = textareas[0].lines()[0].clone();
                                    let new_exec = textareas[1].lines()[0].clone();
                                    let new_icon = textareas[2].lines()[0].clone();

                                    let update_file = {
                                        let file = self.parsed_file.as_mut().unwrap();
                                        file.name = new_name;
                                        file.exec = new_exec;
                                        file.icon = new_icon;
                                        file.clone()
                                    };

                                    utility::save_desktop_file(
                                        &self.items[*file_idx].path,
                                        &update_file,
                                    )?;
                                    self.status_message = Some(format!(
                                        "Saved to ~/.local/share/applications/{}",
                                        self.items[*file_idx].name
                                    ));

                                    let rx = spawn_update_desktop_database();
                                    self.status_job = Some(rx);
                                    self.status_style = Style::default().fg(Color::Green);
                                }
                                _ => {
                                    let input: Input = key.into();
                                    textareas[*active_field].input(input);
                                }
                            },
                        }
                    }
                }
            }
        })
    }

    // Clears Status Message after 2 seconds from footer
    fn clear_status_msg(&mut self) {
        if self.status_job.is_some() {
            return;
        }

        if self
            .status_created
            .is_some_and(|created| created.elapsed() >= Duration::from_secs(2))
        {
            self.status_created = None;
            self.status_message = None;
        }
    }

    // Argghh I am still learning async, forgive me if this code is terrible,
    // So this function is polling for stattus returned after running the binary in a background thread and
    // sets a status message based on output from the thread
    fn poll_status_job(&mut self) {
        if let Some(rx) = &self.status_job {
            use std::sync::mpsc::TryRecvError;
            match rx.try_recv() {
                Ok(result) => {
                    self.status_job = None;

                    let base = self
                        .status_message
                        .clone()
                        .unwrap_or_else(|| "Saved".to_string());

                    match result {
                        DbUpdateResult::Updated => {
                            self.status_message =
                                Some(format!("{} — Desktop database updated", base));
                            self.status_style = Style::default().fg(Color::Green);
                        }
                        DbUpdateResult::MissingBinary => {
                            self.status_message =
                                Some(format!("{} — update-desktop-database not installed", base));
                            self.status_style = Style::default().fg(Color::Yellow);
                        }
                        DbUpdateResult::Failed(err) => {
                            self.status_message =
                                Some(format!("{} — database update failed: {}", base, err));
                            self.status_style = Style::default().fg(Color::Red);
                        }
                    }

                    // start the expiry timer now that the job completed
                    self.status_created = Some(Instant::now());
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.status_job = None;
                }
            }
        }
    }

    fn parse_current(&mut self) {
        if let Some(&real_idx) = self.displaying_indices.get(self.idx) {
            self.parsed_file = self
                .items
                .get(real_idx)
                .and_then(|f| parse_file(&f.path).ok());
        } else {
            self.parsed_file = None
        }
    }
    fn render(&self, frame: &mut Frame, list_state: &mut ListState) {
        let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).spacing(1);
        let [title_area, body_area] = frame.area().layout(&vertical);

        let cols = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .spacing(1)
            .split(body_area);
        let left = cols[0];
        let right = cols[1];

        match &self.mode {
            AppMode::Search { input: query } => {
                let mut box_view = query.clone();
                box_view.set_block(Block::bordered().title("Search"));
                frame.render_widget(&box_view, title_area);
            }

            _ => {
                let text: Vec<Line<'static>> = vec!["  q: quit  /: search  enter: edit".into()];
                let title = Paragraph::new(text).block(
                    ratatui::widgets::Block::default()
                        .title("Desktop File Editor")
                        .borders(ratatui::widgets::Borders::ALL),
                );
                frame.render_widget(title.centered(), title_area);
            }
        }

        let names: Vec<String> = self
            .displaying_indices
            .iter()
            .map(|&i| self.items[i].name.clone())
            .collect();
        list_state.select(Some(self.idx));
        self.render_list(names, frame, left, list_state);

        match self.mode {
            AppMode::Browse => self.render_details(frame, right),
            AppMode::Search { .. } => self.render_details(frame, right),
            AppMode::Edit { .. } => self.render_editor(frame, right),
        }

        let footer_area = Layout::vertical([
            Constraint::Min(0),    // existing body
            Constraint::Length(1), // status line
            Constraint::Length(1), // status line
            Constraint::Length(1), // status line
        ])
        .split(frame.area())[1]; // get the bottom 1-line area

        if let Some(msg) = &self.status_message {
            let paragraph = Paragraph::new(msg.as_str()).style(self.status_style);
            frame.render_widget(paragraph, footer_area);
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

    fn render_details(&self, frame: &mut Frame, area: Rect) {
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
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

        frame.render_widget("Edit Fields (Tab to switch, Esc to cancel)", rows[0]);

        if let AppMode::Edit {
            active_field,
            textareas,
            ..
        } = &self.mode
        {
            let labels = ["Name", "Exec", "Icon"];
            for (i, (ta, label)) in textareas.iter().zip(labels).enumerate() {
                let block = if i == *active_field {
                    Block::bordered()
                        .title(label.to_string())
                        .style(Style::default().fg(Color::Yellow))
                } else {
                    Block::bordered()
                        .title(label.to_string())
                        .style(Style::default().fg(Color::DarkGray))
                };

                let mut ta_clone = ta.clone();
                ta_clone.set_block(block);
                frame.render_widget(&ta_clone, rows[i + 1]);
            }
        }
    }
}
