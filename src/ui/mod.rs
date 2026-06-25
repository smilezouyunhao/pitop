pub mod agent;
pub mod layout;
pub mod logs;
pub mod system;
pub mod tokens;
pub mod tools;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::app::AppState;

const BACKGROUND: Color = Color::Rgb(0, 0, 0);

pub fn render_dashboard(frame: &mut Frame, app: &AppState) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(BACKGROUND)),
        area,
    );

    let areas = layout::dashboard(area);

    render_header(frame, areas.header, app);
    system::render(frame, areas.system, &app.system_stats);
    tokens::render(frame, areas.tokens, &app.session_stats);
    agent::render(frame, areas.agent, app);
    tools::render(frame, areas.tools, &app.session_stats);
    logs::render(frame, areas.logs, app);
    render_footer(frame, areas.footer);

    if app.session_picker_open {
        render_session_picker(frame, frame.area(), app);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &AppState) {
    let session = app
        .session_stats
        .session_id
        .as_deref()
        .unwrap_or("no-session");
    let model = app
        .session_stats
        .current_model
        .as_deref()
        .unwrap_or("unknown-model");

    let header = Paragraph::new(Line::from(vec![
        Span::styled("pitop", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  |  "),
        Span::raw(format!("session: {session}")),
        Span::raw("  |  "),
        Span::raw(format!("model: {model}")),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(header, area);
}

fn render_session_picker(frame: &mut Frame, area: Rect, app: &AppState) {
    let popup = centered_rect(80, 60, area);
    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = if app.session_choices.is_empty() {
        vec![ListItem::new("No sessions found")]
    } else {
        app.session_choices
            .iter()
            .enumerate()
            .map(|(index, choice)| {
                let marker = if index == app.selected_session_index {
                    "›"
                } else {
                    " "
                };
                let mut item = ListItem::new(Line::from(vec![
                    Span::raw(format!("{marker} ")),
                    Span::styled(
                        choice.id.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!("  {}  {}", choice.timestamp, choice.cwd)),
                ]));

                if index == app.selected_session_index {
                    item = item.style(Style::default().fg(Color::Black).bg(Color::Cyan));
                }

                item
            })
            .collect()
    };

    let list = List::new(items)
        .style(Style::default().bg(BACKGROUND))
        .block(
            Block::default()
                .title("Sessions  ↑/↓ select  Enter load  Esc close")
                .borders(Borders::ALL)
                .style(Style::default().bg(BACKGROUND))
                .border_style(Style::default().fg(Color::Cyan)),
        );
    frame.render_widget(list, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);

    horizontal[1]
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new("q: quit  |  Tab: focus  |  arrows: scroll logs  |  s: sessions")
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    frame.render_widget(footer, area);
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_dashboard_without_panic() {
        let backend = TestBackend::new(100, 32);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        let app = AppState::new();

        terminal
            .draw(|frame| render_dashboard(frame, &app))
            .expect("draw dashboard");
    }
}
