use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::app::AppState;

pub fn render(frame: &mut Frame, area: Rect, app: &AppState) {
    let height = area.height.saturating_sub(2) as usize;
    let items: Vec<ListItem> = app
        .logs
        .iter()
        .rev()
        .take(height)
        .rev()
        .map(|entry| {
            let timestamp = entry.timestamp.as_deref().unwrap_or("--");
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{timestamp}] "),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(entry.message.clone()),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("┤ LOG STREAM ├")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray)),
    );
    frame.render_widget(list, area);
}
