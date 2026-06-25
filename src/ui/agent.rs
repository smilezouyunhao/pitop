use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::AppState;

pub fn render(frame: &mut Frame, area: Rect, app: &AppState) {
    let stats = &app.session_stats;
    let session = stats.session_id.as_deref().unwrap_or("No session found");
    let model = match (&stats.current_provider, &stats.current_model) {
        (Some(provider), Some(model)) => format!("{provider}/{model}"),
        (_, Some(model)) => model.clone(),
        _ => "unknown".to_owned(),
    };
    let thinking = stats.thinking_level.as_deref().unwrap_or("unknown");
    let cwd = stats.cwd.as_deref().unwrap_or("unknown cwd");
    let file = app
        .current_session_path
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("unknown file");

    let text = format!(
        "Status: ● Running   Session: {session}\nModel: {model}   Thinking: {thinking}   Messages: {}   Compactions: {}\nCwd: {cwd}   File: {file}",
        stats.message_count, stats.compactions
    );

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .title("AGENT STATUS")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(paragraph, area);
}
