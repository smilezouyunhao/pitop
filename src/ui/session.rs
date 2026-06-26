use std::path::Path;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    app::AppState,
    data::{process::PiInstance, session::SessionStats},
};

#[derive(Debug, Clone)]
struct SessionColumn {
    key: &'static str,
    header: &'static str,
    min_width: usize,
    width: usize,
}

#[derive(Debug, Clone)]
struct SessionRow {
    selected: bool,
    pid: String,
    project: String,
    session: String,
    summary: String,
    status: String,
    model: String,
    context: String,
    tokens: String,
    memory: String,
    turn: String,
}

pub fn render(frame: &mut Frame, area: Rect, app: &AppState) {
    let inner_width = area.width.saturating_sub(2) as usize;
    let columns = session_columns(inner_width);
    let max_rows = area.height.saturating_sub(3) as usize;
    let rows = session_rows(app, max_rows);

    let mut lines = vec![header_line(&columns)];
    if rows.is_empty() {
        lines.push(Line::from(Span::styled(
            "No active Pi processes",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.extend(rows.iter().map(|row| value_line(&columns, row)));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title("┤ SESSION ├")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(paragraph, area);
}

pub fn row_count(app: &AppState) -> usize {
    associated_instances(app)
        .count()
        .max(usize::from(app.session_stats.session_id.is_some()))
        .max(1)
}

fn session_rows(app: &AppState, limit: usize) -> Vec<SessionRow> {
    if limit == 0 {
        return Vec::new();
    }

    let associated: Vec<_> = associated_instances(app).take(limit).collect();
    if !associated.is_empty() {
        return associated
            .into_iter()
            .enumerate()
            .map(|(index, instance)| {
                row_from_instance(instance, index == app.selected_instance_index)
            })
            .collect();
    }

    if app.session_stats.session_id.is_some() {
        return vec![row_from_stats(&app.session_stats, true)];
    }

    Vec::new()
}

fn associated_instances(app: &AppState) -> impl Iterator<Item = &PiInstance> {
    app.pi_instances
        .iter()
        .filter(|instance| instance.session_path.is_some() || instance.stats.is_some())
}

fn row_from_instance(instance: &PiInstance, selected: bool) -> SessionRow {
    let stats = instance.stats.as_ref();
    let project = stats
        .and_then(|stats| stats.cwd.as_deref())
        .and_then(project_name)
        .map(str::to_owned)
        .or_else(|| {
            project_name_from_session_path(instance.session_path.as_deref()).map(str::to_owned)
        })
        .unwrap_or_else(|| "-".to_owned());

    SessionRow {
        selected,
        pid: instance.pid.to_string(),
        project,
        session: stats
            .and_then(|stats| stats.session_id.as_deref())
            .map(short_session_id)
            .unwrap_or_else(|| "-".to_owned()),
        summary: "-".to_owned(),
        status: "● Live".to_owned(),
        model: stats
            .and_then(|stats| stats.current_model.as_deref())
            .unwrap_or("-")
            .to_owned(),
        context: stats
            .map(|stats| {
                context_percent_label(stats.latest_context_tokens, stats.current_model.as_deref())
            })
            .unwrap_or_else(|| "-".to_owned()),
        tokens: stats
            .map(|stats| format_count(stats.tokens.total_tokens))
            .unwrap_or_else(|| "-".to_owned()),
        memory: format_memory(instance.memory_bytes),
        turn: stats
            .map(|stats| stats.turn_count.to_string())
            .unwrap_or_else(|| "-".to_owned()),
    }
}

fn row_from_stats(stats: &SessionStats, selected: bool) -> SessionRow {
    SessionRow {
        selected,
        pid: "-".to_owned(),
        project: stats
            .cwd
            .as_deref()
            .and_then(project_name)
            .unwrap_or("-")
            .to_owned(),
        session: stats
            .session_id
            .as_deref()
            .map(short_session_id)
            .unwrap_or_else(|| "-".to_owned()),
        summary: "-".to_owned(),
        status: "● Wait".to_owned(),
        model: stats.current_model.as_deref().unwrap_or("-").to_owned(),
        context: context_percent_label(stats.latest_context_tokens, stats.current_model.as_deref()),
        tokens: format_count(stats.tokens.total_tokens),
        memory: "-".to_owned(),
        turn: stats.turn_count.to_string(),
    }
}

fn session_columns(available_width: usize) -> Vec<SessionColumn> {
    let mut columns = vec![
        column("selected", "", 2, 2),
        column("pid", "Pid", 5, 7),
        column("project", "Project", 10, 16),
        column("session", "Session", 9, 10),
        column("summary", "Summary", 8, 12),
        column("status", "Status", 8, 10),
        column("model", "Model", 10, 16),
        column("context", "Context", 8, 8),
        column("tokens", "Tokens", 10, 13),
        column("memory", "Memory", 7, 9),
        column("turn", "Turn", 5, 5),
    ];

    fit_columns(&mut columns, available_width);
    expand_columns(&mut columns, available_width);
    columns
}

fn column(
    key: &'static str,
    header: &'static str,
    min_width: usize,
    width: usize,
) -> SessionColumn {
    SessionColumn {
        key,
        header,
        min_width,
        width,
    }
}

fn fit_columns(columns: &mut Vec<SessionColumn>, available_width: usize) {
    shrink_columns(columns, available_width);

    for key in ["memory", "context", "summary", "turn", "model", "pid"] {
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

fn value_line(columns: &[SessionColumn], row: &SessionRow) -> Line<'static> {
    let spans = columns
        .iter()
        .map(|column| {
            let mut style = Style::default()
                .fg(cell_color(column.key))
                .add_modifier(Modifier::BOLD);
            if row.selected {
                style = style.bg(Color::Rgb(24, 24, 42));
            }
            Span::styled(pad_or_truncate(row.value(column.key), column.width), style)
        })
        .collect::<Vec<_>>();

    Line::from(spans)
}

impl SessionRow {
    fn value(&self, key: &str) -> &str {
        match key {
            "selected" => {
                if self.selected {
                    "›"
                } else {
                    ""
                }
            }
            "pid" => &self.pid,
            "project" => &self.project,
            "session" => &self.session,
            "summary" => &self.summary,
            "status" => &self.status,
            "model" => &self.model,
            "context" => &self.context,
            "tokens" => &self.tokens,
            "memory" => &self.memory,
            "turn" => &self.turn,
            _ => "-",
        }
    }
}

fn cell_color(key: &str) -> Color {
    match key {
        "selected" => Color::Cyan,
        "pid" | "summary" | "memory" => Color::DarkGray,
        "session" | "status" => Color::Yellow,
        "context" => Color::LightBlue,
        "tokens" => Color::Magenta,
        "turn" => Color::Cyan,
        _ => Color::White,
    }
}

fn context_percent_label(context_tokens: Option<u64>, model: Option<&str>) -> String {
    let Some(context_tokens) = context_tokens else {
        return "-".to_owned();
    };
    let Some(model) = model else {
        return "-".to_owned();
    };
    let Some(context_window) = context_window_for_model(model) else {
        return "-".to_owned();
    };

    let percent = ((context_tokens as f64 / context_window as f64) * 100.0).clamp(0.0, 999.0);
    format!("{percent:.0}%")
}

fn context_window_for_model(model: &str) -> Option<u64> {
    let normalized = model.to_ascii_lowercase();

    if normalized.contains("gpt-5.5") {
        Some(272_000)
    } else if normalized.contains("gpt-5.1")
        || normalized.contains("gpt-5-mini")
        || normalized.contains("gpt-5-codex")
    {
        Some(128_000)
    } else if normalized.contains("claude-opus-4-8") || normalized.contains("claude-sonnet-4-6") {
        Some(1_000_000)
    } else if normalized.contains("deepseek-v4-flash") || normalized.contains("deepseek-v4-pro") {
        Some(1_000_000)
    } else if normalized.contains("claude") {
        Some(200_000)
    } else {
        None
    }
}

fn project_name(cwd: &str) -> Option<&str> {
    Path::new(cwd).file_name()?.to_str()
}

fn project_name_from_session_path(path: Option<&Path>) -> Option<&str> {
    path?
        .parent()?
        .file_name()?
        .to_str()?
        .trim_matches('-')
        .rsplit('-')
        .next()
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

fn format_memory(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{}M", bytes / MB)
    } else {
        format!("{}K", bytes / 1024)
    }
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
    fn formats_context_percent_for_known_model() {
        assert_eq!(context_percent_label(Some(136_000), Some("gpt-5.5")), "50%");
        assert_eq!(
            context_percent_label(Some(500_000), Some("deepseek-v4-pro")),
            "50%"
        );
    }

    #[test]
    fn hides_context_percent_for_unknown_model() {
        assert_eq!(context_percent_label(Some(10_000), Some("unknown")), "-");
        assert_eq!(context_percent_label(None, Some("gpt-5.5")), "-");
    }

    #[test]
    fn formats_memory_compactly() {
        assert_eq!(format_memory(512 * 1024), "512K");
        assert_eq!(format_memory(335 * 1024 * 1024), "335M");
        assert_eq!(format_memory(1536 * 1024 * 1024), "1.5G");
    }

    #[test]
    fn session_columns_fit_available_width() {
        let columns = session_columns(72);
        assert!(total_width(&columns) <= 72);
    }

    #[test]
    fn session_columns_expand_to_available_width() {
        let columns = session_columns(160);
        assert_eq!(total_width(&columns), 160);
    }

    #[test]
    fn narrow_session_columns_keep_core_fields() {
        let columns = session_columns(48);
        let keys: Vec<&str> = columns.iter().map(|column| column.key).collect();

        assert!(keys.contains(&"project"));
        assert!(keys.contains(&"session"));
        assert!(keys.contains(&"status"));
        assert!(keys.contains(&"tokens"));
        assert!(total_width(&columns) <= 48);
    }

    #[test]
    fn ignores_unassociated_process_candidates() {
        let mut app = AppState::new();
        app.pi_instances.push(PiInstance {
            pid: 42,
            ppid: 1,
            memory_bytes: 335 * 1024 * 1024,
            cpu_percent: 0.0,
            command: "pi-coding-agent".to_owned(),
            session_path: None,
            stats: None,
        });

        let rows = session_rows(&app, 8);

        assert!(rows.is_empty());
        assert_eq!(row_count(&app), 1);
    }

    #[test]
    fn uses_associated_instances_before_fallback_stats() {
        let mut app = AppState::new();
        app.pi_instances.push(PiInstance {
            pid: 42,
            ppid: 1,
            memory_bytes: 335 * 1024 * 1024,
            cpu_percent: 0.0,
            command: "pi-coding-agent".to_owned(),
            session_path: Some("session.jsonl".into()),
            stats: None,
        });

        let rows = session_rows(&app, 8);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pid, "42");
        assert_eq!(rows[0].memory, "335M");
    }
}
