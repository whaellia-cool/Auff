use chrono::{Datelike, NaiveDate, Utc};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::*,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use tui_input::{backend::crossterm::EventHandler, Input};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum MediaType {
    Movie,
    Book,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DayEntry {
    content: Option<(MediaType, String)>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct YearData {
    days: HashMap<NaiveDate, DayEntry>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct AppData {
    years: HashMap<i32, YearData>,
}

struct App {
    data: AppData,
    current_year: i32,
    table_state: TableState,
    active_type: MediaType,
    input: Input,
    input_mode: bool,
    save_path: String,
}

impl App {
    fn new() -> Self {
        let mut app = App {
            data: AppData::default(),
            current_year: Utc::now().year(),
            table_state: TableState::default(),
            active_type: MediaType::Movie,
            input: Input::default(),
            input_mode: false,
            save_path: "media_calendar.ron".to_string(),
        };
        app.load();
        app.table_state.select(Some(0));
        app.table_state.select_column(Some(1));
        app
    }

    fn load(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.save_path) {
            if let Ok(data) = ron::from_str(&content) {
                self.data = data;
            }
        }
    }

    fn save(&self) {
        if let Ok(content) = ron::ser::to_string_pretty(&self.data, ron::ser::PrettyConfig::default()) {
            let _ = fs::write(&self.save_path, content);
        }
    }

    fn add_entry(&mut self, title: String) {
        if title.trim().is_empty() { return; }
        let day = self.table_state.selected().unwrap_or(0) as u32 + 1;
        let month = self.table_state.selected_column().unwrap_or(1) as u32;

        if let Some(date) = NaiveDate::from_ymd_opt(self.current_year, month, day) {
            let year_data = self.data.years.entry(self.current_year).or_default();
            let entry = year_data.days.entry(date).or_default();
            entry.content = Some((self.active_type, title));
            self.save();
        }
    }

    fn delete_entry(&mut self) {
        let day = self.table_state.selected().unwrap_or(0) as u32 + 1;
        let month = self.table_state.selected_column().unwrap_or(1) as u32;
        if let Some(date) = NaiveDate::from_ymd_opt(self.current_year, month, day) {
            if let Some(year_data) = self.data.years.get_mut(&self.current_year) {
                year_data.days.remove(&date);
                self.save();
            }
        }
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let mut app = App::new();
    
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press { continue; }

            if app.input_mode {
                match key.code {
                    KeyCode::Enter => {
                        app.add_entry(app.input.value().to_string());
                        app.input.reset();
                        app.input_mode = false;
                    }
                    KeyCode::Esc => {
                        app.input.reset();
                        app.input_mode = false;
                    }
                    _ => { app.input.handle_event(&Event::Key(key)); }
                }
            } else {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) => break,
                    (KeyCode::Char('m'), _) => {
                        app.active_type = MediaType::Movie;
                        app.input_mode = true;
                    }
                    (KeyCode::Char('b'), _) => {
                        app.active_type = MediaType::Book;
                        app.input_mode = true;
                    }
                    (KeyCode::Char('x') | KeyCode::Delete, _) => app.delete_entry(),
                    (KeyCode::Right, _) => { let _ = app.table_state.select_next_column(); }
                    (KeyCode::Left, _)  => { let _ = app.table_state.select_previous_column(); }
                    (KeyCode::Down, _)  => { let _ = app.table_state.select_next(); }
                    (KeyCode::Up, _)    => { let _ = app.table_state.select_previous(); }
                    (KeyCode::Tab, KeyModifiers::NONE) => { app.current_year += 1; }
                    (KeyCode::Tab, KeyModifiers::SHIFT) | (KeyCode::BackTab, _) => { app.current_year -= 1; }
                    _ => {}
                }
            }
        }
    }
    ratatui::restore();
    Ok(())
}

fn ui(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let months = ["", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let header = Row::new(months.iter().map(|&m| Cell::from(m).style(Style::default().fg(Color::Cyan).bold())));

    let mut full_active_title = String::new();
    let mut rows = vec![];

    for day in 1..=31u32 {
        let mut cells = vec![Cell::from(format!("{:2}", day)).style(Style::default().bold())];
        for month in 1..=12u32 {
            let mut content = Span::raw("");
            
            if let Some(date) = NaiveDate::from_ymd_opt(app.current_year, month, day) {
                if let Some(entry) = app.data.years.get(&app.current_year).and_then(|y| y.days.get(&date)) {
                    if let Some((kind, title)) = &entry.content {
                        let color = match kind {
                            MediaType::Movie => Color::Magenta,
                            MediaType::Book  => Color::Green,
                        };

                        let is_selected = app.table_state.selected() == Some(day as usize - 1) 
                                          && app.table_state.selected_column() == Some(month as usize);
                        
                        if is_selected {
                            full_active_title = title.clone();
                        }

                        let display_text = if title.chars().count() > 12 {
                            format!("{:.10}..", title)
                        } else {
                            title.clone()
                        };
                        content = Span::styled(display_text, Style::default().fg(color));
                    }
                }
            } else if day > 28 {
                content = Span::styled("---", Style::default().fg(Color::DarkGray));
            }
            cells.push(Cell::from(Line::from(content)));
        }
        rows.push(Row::new(cells));
    }

    let widths = std::iter::once(Constraint::Length(4))
        .chain(std::iter::repeat(Constraint::Length(14)).take(12))
        .collect::<Vec<_>>();

    let block_title = if full_active_title.is_empty() {
        format!(" {} ", app.current_year)
    } else {
        format!(" {} │ {} ", app.current_year, full_active_title)
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(block_title)
            .title_alignment(Alignment::Center))
        .cell_highlight_style(Style::default().bg(Color::Rgb(100, 100, 100)).fg(Color::White).bold());

    frame.render_stateful_widget(table, chunks[0], &mut app.table_state);

    let help_text = " M: Movie | B: Book | X: Delete | Tab: Year | Q: Quit ";
    let help_bar = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Blue))
        .alignment(Alignment::Center);
    frame.render_widget(help_bar, chunks[1]);

    if app.input_mode {
        let title = format!(" Add {} ", if app.active_type == MediaType::Movie { "Movie" } else { "Book" });
        render_popup(frame, title, app.input.value(), Color::Cyan, 3);
    }
}

fn render_popup(frame: &mut Frame, title: String, content: &str, color: Color, h: u16) {
    let area = centered_rect(50, h * 10, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(content)
            .block(Block::default().title(title).borders(Borders::ALL).border_style(Style::default().fg(color)))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
