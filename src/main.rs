mod utility;

use std::path::PathBuf;

use crossterm::event::{self, KeyCode};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Stylize},
    text::{Line, Span},
    widgets::{List, ListState},
};

use crate::utility::load_files;

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
    let mut app = App {
        items: load_files()?,
        idx: 0,
        parsed_file: None,
        mode: AppMode::Browse,
    };

    ratatui::run(|terminal| {
        loop {
            terminal.draw(|frame| render(&mut app, frame, &mut list_state))?;
            if let Some(key) = event::read()?.as_key_press_event() {
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        app.idx = (app.idx + 1).min(app.items.len().saturating_sub(1));
                    }
                    KeyCode::Char('k') | KeyCode::Up => app.idx = (app.idx - 1).saturating_sub(1),
                    KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                    _ => {}
                }
            }
        }
    })
}

fn render(app: &mut App, frame: &mut Frame, list_state: &mut ListState) {
    let constraints = [Constraint::Length(1), Constraint::Fill(1)];
    let layout = Layout::vertical(constraints).spacing(1);
    let [top, first] = frame.area().layout(&layout);

    let title = Line::from_iter([
        Span::from("List Widget").bold(),
        Span::from(" (Press 'q' to quit and arrow keys to navigate)"),
    ]);
    frame.render_widget(title.centered(), top);

    let names: Vec<String> = app.items.iter().map(|f| f.name.clone()).collect();

    list_state.select(Some(app.idx));
    render_list(names, frame, first, list_state);
}

fn render_list(items: Vec<String>, frame: &mut Frame, area: Rect, list_state: &mut ListState) {
    let list = List::new(items)
        .style(Color::White)
        .highlight_style(Modifier::REVERSED)
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, list_state);
}
