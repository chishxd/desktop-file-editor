use std::path::PathBuf;

use crossterm::event::{self, KeyCode};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Stylize},
    text::{Line, Span},
    widgets::{List, ListState},
};

struct File {
    name: String,
    path: PathBuf,
}

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
    items: Vec<File>,
    idx: usize,
    parsed_file: Option<ParsedDesktopFile>,
    mode: AppMode,
}

fn main() -> std::io::Result<()> {
    let mut list_state = ListState::default().with_selected(Some(0));
    ratatui::run(|terminal| {
        loop {
            terminal.draw(|frame| render(frame, &mut list_state))?;
            if let Some(key) = event::read()?.as_key_press_event() {
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => list_state.select_next(),
                    KeyCode::Char('k') | KeyCode::Up => list_state.select_previous(),
                    KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                    _ => {}
                }
            }
        }
    })
}

fn render(frame: &mut Frame, list_state: &mut ListState) {
    let constraints = [Constraint::Length(1), Constraint::Fill(1)];
    let layout = Layout::vertical(constraints).spacing(1);
    let [top, first] = frame.area().layout(&layout);

    let title = Line::from_iter([
        Span::from("List Widget").bold(),
        Span::from(" (Press 'q' to quit and arrow keys to navigate)"),
    ]);
    frame.render_widget(title.centered(), top);

    render_list(frame, first, list_state);
}

fn render_list(frame: &mut Frame, area: Rect, list_state: &mut ListState) {
    let items = ["App 1", "App 2", "App 3", "App 4"];

    let list = List::new(items)
        .style(Color::White)
        .highlight_style(Modifier::REVERSED)
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, list_state);
}
