use ratatui::{
    Frame,
    layout::Rect,
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

    let text = format!(
        "Status: ● Running   Session: {session}\nModel: {model}   Thinking: {thinking}   Messages: {}   Compactions: {}",
        stats.message_count, stats.compactions
    );

    let paragraph =
        Paragraph::new(text).block(Block::default().title("AGENT STATUS").borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}
