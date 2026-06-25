use std::path::Path;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::AppState;

#[derive(Debug, Clone)]
struct SessionColumn {
    key: &'static str,
    header: &'static str,
    value: String,
    color: Color,
    min_width: usize,
    width: usize,
}

pub fn render(frame: &mut Frame, area: Rect, app: &AppState) {
    let inner_width = area.width.saturating_sub(2) as usize;
    let columns = session_columns(app, inner_width);
    let lines = vec![header_line(&columns), value_line(&columns)];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title("┤ SESSION ├")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(paragraph, area);
}

fn session_columns(app: &AppState, available_width: usize) -> Vec<SessionColumn> {
    let stats = &app.session_stats;
    let project = stats
        .cwd
        .as_deref()
        .and_then(project_name)
        .unwrap_or("-")
        .to_owned();
    let session = stats
        .session_id
        .as_deref()
        .map(short_session_id)
        .unwrap_or_else(|| "-".to_owned());
    let status = if stats.session_id.is_some() {
        "● Wait".to_owned()
    } else {
        "-".to_owned()
    };
    let model = stats.current_model.as_deref().unwrap_or("-").to_owned();
    let tokens = format_count(stats.tokens.total_tokens);
    let turn = stats.message_count.to_string();

    let mut columns = vec![
        column("pid", "Pid", "-", Color::DarkGray, 5, 7),
        column("project", "Project", project, Color::White, 10, 16),
        column("session", "Session", session, Color::Yellow, 9, 10),
        column("summary", "Summary", "-", Color::DarkGray, 8, 12),
        column("status", "Status", status, Color::Yellow, 8, 10),
        column("model", "Model", model, Color::White, 10, 16),
        column("context", "Context", "-", Color::DarkGray, 8, 8),
        column("tokens", "Tokens", tokens, Color::Magenta, 10, 13),
        column("memory", "Memory", "-", Color::DarkGray, 7, 9),
        column("turn", "Turn", turn, Color::Cyan, 5, 5),
    ];

    fit_columns(&mut columns, available_width);
    expand_columns(&mut columns, available_width);
    columns
}

fn column(
    key: &'static str,
    header: &'static str,
    value: impl Into<String>,
    color: Color,
    min_width: usize,
    width: usize,
) -> SessionColumn {
    SessionColumn {
        key,
        header,
        value: value.into(),
        color,
        min_width,
        width,
    }
}

fn fit_columns(columns: &mut Vec<SessionColumn>, available_width: usize) {
    shrink_columns(columns, available_width);

    for key in ["pid", "memory", "context", "summary", "turn", "model"] {
        if total_width(columns) <= available_width {
            break;
        }
        if let Some(index) = columns.iter().position(|column| column.key == key) {
            columns.remove(index);
            shrink_columns(columns, available_width);
        }
    }
}

fn shrink_columns(columns: &mut [SessionColumn], available_width: usize) {
    while total_width(columns) > available_width {
        let Some(column) = columns
            .iter_mut()
            .find(|column| column.width > column.min_width)
        else {
            break;
        };
        column.width -= 1;
    }
}

fn expand_columns(columns: &mut [SessionColumn], available_width: usize) {
    let mut remaining = available_width.saturating_sub(total_width(columns));
    if remaining == 0 {
        return;
    }

    for key in ["summary", "model", "project", "tokens", "status"] {
        let Some(column) = columns.iter_mut().find(|column| column.key == key) else {
            continue;
        };

        let add = remaining.min(match key {
            "summary" => 32,
            "model" => 16,
            "project" => 12,
            _ => 8,
        });
        column.width += add;
        remaining -= add;

        if remaining == 0 {
            break;
        }
    }

    if remaining > 0 {
        if let Some(column) = columns.last_mut() {
            column.width += remaining;
        }
    }
}

fn total_width(columns: &[SessionColumn]) -> usize {
    columns.iter().map(|column| column.width).sum()
}

fn header_line(columns: &[SessionColumn]) -> Line<'static> {
    Line::from(
        columns
            .iter()
            .map(|column| {
                Span::styled(
                    pad_or_truncate(column.header, column.width),
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::BOLD),
                )
            })
            .collect::<Vec<_>>(),
    )
}

fn value_line(columns: &[SessionColumn]) -> Line<'static> {
    Line::from(
        columns
            .iter()
            .map(|column| {
                Span::styled(
                    pad_or_truncate(&column.value, column.width),
                    Style::default()
                        .fg(column.color)
                        .add_modifier(Modifier::BOLD),
                )
            })
            .collect::<Vec<_>>(),
    )
}

fn project_name(cwd: &str) -> Option<&str> {
    Path::new(cwd).file_name()?.to_str()
}

fn short_session_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn pad_or_truncate(value: &str, width: usize) -> String {
    let mut truncated: String = value.chars().take(width).collect();
    if value.chars().count() > width && width > 1 {
        truncated.pop();
        truncated.push('~');
    }
    format!("{truncated:<width$}")
}

fn format_count(value: u64) -> String {
    let text = value.to_string();
    let mut out = String::new();

    for (index, ch) in text.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }

    out.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortens_session_id() {
        assert_eq!(short_session_id("019ef9ea-c348"), "019ef9ea");
    }

    #[test]
    fn truncates_long_cells_with_marker() {
        assert_eq!(pad_or_truncate("codex-buddy", 6), "codex~");
    }

    #[test]
    fn formats_counts_with_commas() {
        assert_eq!(format_count(21_100_844), "21,100,844");
    }

    #[test]
    fn session_columns_fit_available_width() {
        let app = AppState::new();
        let columns = session_columns(&app, 72);
        assert!(total_width(&columns) <= 72);
    }

    #[test]
    fn session_columns_expand_to_available_width() {
        let app = AppState::new();
        let columns = session_columns(&app, 160);
        assert_eq!(total_width(&columns), 160);
    }

    #[test]
    fn narrow_session_columns_keep_core_fields() {
        let app = AppState::new();
        let columns = session_columns(&app, 48);
        let keys: Vec<&str> = columns.iter().map(|column| column.key).collect();

        assert!(keys.contains(&"project"));
        assert!(keys.contains(&"session"));
        assert!(keys.contains(&"status"));
        assert!(keys.contains(&"tokens"));
        assert!(total_width(&columns) <= 48);
    }
}
