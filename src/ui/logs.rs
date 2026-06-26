use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::app::{AppLogEntry, AppState, LogKind};

pub fn render(frame: &mut Frame, area: Rect, app: &AppState) {
    let height = area.height.saturating_sub(2) as usize;
    let items: Vec<ListItem> = app
        .logs
        .iter()
        .rev()
        .take(height)
        .rev()
        .map(log_item)
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("┤ LOG STREAM ├")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray)),
    );
    frame.render_widget(list, area);
}

fn log_item(entry: &AppLogEntry) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(
            format!("{:<7} ", label(entry.kind)),
            Style::default()
                .fg(color(entry.kind))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            entry.message.clone(),
            Style::default().fg(color(entry.kind)),
        ),
    ]))
}

fn label(kind: LogKind) -> &'static str {
    match kind {
        LogKind::User => "USER",
        LogKind::ToolCall => "TOOL",
        LogKind::ToolResult => "RESULT",
        LogKind::Command => "CMD",
        LogKind::Test => "TEST",
        LogKind::Error => "ERROR",
        LogKind::Model => "MODEL",
        LogKind::Compact => "COMPACT",
        LogKind::Done => "DONE",
        LogKind::Other => "INFO",
    }
}

fn color(kind: LogKind) -> Color {
    match kind {
        LogKind::User => Color::LightCyan,
        LogKind::ToolCall => Color::Magenta,
        LogKind::ToolResult => Color::Green,
        LogKind::Command => Color::LightBlue,
        LogKind::Test => Color::Yellow,
        LogKind::Error => Color::Red,
        LogKind::Model => Color::Cyan,
        LogKind::Compact => Color::LightYellow,
        LogKind::Done => Color::Green,
        LogKind::Other => Color::Gray,
    }
}
