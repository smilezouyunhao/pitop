pub mod agent;
pub mod layout;
pub mod logs;
pub mod system;
pub mod tokens;
pub mod tools;

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::AppState;

pub fn render_dashboard(frame: &mut Frame, app: &AppState) {
    let areas = layout::dashboard(frame.area());

    render_header(frame, areas.header, app);
    system::render(frame, areas.system, &app.system_stats);
    tokens::render(frame, areas.tokens, &app.session_stats);
    agent::render(frame, areas.agent, app);
    tools::render(frame, areas.tools, &app.session_stats);
    logs::render(frame, areas.logs, app);
    render_footer(frame, areas.footer);
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
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new("q: quit  |  Tab: focus  |  arrows: scroll logs  |  s: sessions")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

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
