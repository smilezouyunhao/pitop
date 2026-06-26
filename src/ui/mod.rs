pub mod layout;
pub mod logs;
pub mod session;
pub mod system;
pub mod tokens;
pub mod tools;

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::AppState;

const BACKGROUND: Color = Color::Rgb(0, 0, 0);

pub fn render_dashboard(frame: &mut Frame, app: &AppState) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(BACKGROUND)),
        area,
    );

    let areas = layout::dashboard(area, session::row_count(app));

    render_header(frame, areas.header, app);
    system::render(frame, areas.system, &app.system_stats);
    tokens::render(frame, areas.tokens, &app.session_stats);
    tools::render(frame, areas.tools, &app.session_stats);
    session::render(frame, areas.session, app);
    logs::render(frame, areas.logs, app);
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
