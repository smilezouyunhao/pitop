use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::data::session::SessionStats;

const BASIC_TOOLS: [&str; 4] = ["bash", "read", "write", "edit"];
const NAME_WIDTH: usize = 12;
const COUNT_WIDTH: usize = 7;
const COLUMN_GAP: &str = "    ";

pub fn render(frame: &mut Frame, area: Rect, stats: &SessionStats) {
    let max_rows = area.height.saturating_sub(4) as usize;
    let basic_tools = basic_tool_counts(stats);
    let other_tools = other_tool_counts(stats, max_rows);
    let total_calls: u64 = stats.tool_counts.values().sum();
    let unique_tools = stats.tool_counts.len();

    let mut lines = vec![summary_line(total_calls, unique_tools)];

    if total_calls == 0 {
        lines.push(Line::from(Span::styled(
            "No tool calls",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.push(columns_header());
        lines.extend((0..max_rows).map(|index| {
            let left = basic_tools.get(index);
            let right = other_tools.get(index);
            paired_tool_line(left, right)
        }));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title("┤ TOOL CALLS ├")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta)),
    );
    frame.render_widget(paragraph, area);
}

fn summary_line(total_calls: u64, unique_tools: usize) -> Line<'static> {
    Line::from(vec![
        Span::raw("Total "),
        Span::styled(
            format_count(total_calls),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   Unique "),
        Span::styled(
            unique_tools.to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

fn columns_header() -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{:<width$}", "Basic", width = NAME_WIDTH + COUNT_WIDTH),
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(COLUMN_GAP),
        Span::styled(
            "Other",
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

fn paired_tool_line(left: Option<&(String, u64)>, right: Option<&(String, u64)>) -> Line<'static> {
    let mut spans = Vec::new();
    spans.extend(tool_spans(left, Color::LightBlue));
    spans.push(Span::raw(COLUMN_GAP));
    spans.extend(tool_spans(right, Color::Cyan));
    Line::from(spans)
}

fn tool_spans(tool: Option<&(String, u64)>, color: Color) -> Vec<Span<'static>> {
    match tool {
        Some((name, count)) => vec![
            Span::styled(
                format!("{name:<NAME_WIDTH$}"),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>COUNT_WIDTH$}", format_count(*count)),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
        ],
        None => vec![Span::raw(" ".repeat(NAME_WIDTH + COUNT_WIDTH))],
    }
}

fn basic_tool_counts(stats: &SessionStats) -> Vec<(String, u64)> {
    BASIC_TOOLS
        .iter()
        .map(|name| {
            (
                (*name).to_owned(),
                *stats.tool_counts.get(*name).unwrap_or(&0),
            )
        })
        .collect()
}

fn other_tool_counts(stats: &SessionStats, limit: usize) -> Vec<(String, u64)> {
    if limit == 0 {
        return Vec::new();
    }

    let mut tools: Vec<(String, u64)> = stats
        .tool_counts
        .iter()
        .filter(|(name, _)| !BASIC_TOOLS.contains(&name.as_str()))
        .map(|(name, count)| (display_label(name), *count))
        .collect();

    tools.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    if tools.len() <= limit {
        return tools;
    }

    let more_count: u64 = tools.iter().skip(limit - 1).map(|(_, count)| *count).sum();
    tools.truncate(limit - 1);
    tools.push(("More".to_owned(), more_count));
    tools
}

fn display_label(name: &str) -> String {
    match name {
        "obsidian_list_notes" => "obs_notes".to_owned(),
        "obsidian_list_vaults" => "obs_vault".to_owned(),
        "obsidian_read" => "obs_read".to_owned(),
        "obsidian_write" => "obs_write".to_owned(),
        "obsidian_search" => "obs_search".to_owned(),
        "obsidian_append" => "obs_append".to_owned(),
        _ => truncate_label(name, NAME_WIDTH),
    }
}

fn truncate_label(name: &str, max_chars: usize) -> String {
    let mut chars = name.chars();
    let mut label: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() && max_chars > 1 {
        label.pop();
        label.push('~');
    }
    label
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
    fn basic_tools_are_fixed_and_ordered() {
        let mut stats = SessionStats::default();
        stats.tool_counts.insert("read".to_owned(), 2);
        stats.tool_counts.insert("bash".to_owned(), 5);

        let tools = basic_tool_counts(&stats);

        assert_eq!(
            tools,
            vec![
                ("bash".to_owned(), 5),
                ("read".to_owned(), 2),
                ("write".to_owned(), 0),
                ("edit".to_owned(), 0),
            ]
        );
    }

    #[test]
    fn other_tools_exclude_basic_tools_and_collapse_overflow() {
        let mut stats = SessionStats::default();
        stats.tool_counts.insert("bash".to_owned(), 5);
        stats.tool_counts.insert("obsidian_read".to_owned(), 2);
        stats
            .tool_counts
            .insert("obsidian_list_vaults".to_owned(), 1);
        stats.tool_counts.insert("custom".to_owned(), 1);

        let tools = other_tool_counts(&stats, 2);

        assert_eq!(
            tools,
            vec![("obs_read".to_owned(), 2), ("More".to_owned(), 2)]
        );
    }

    #[test]
    fn creates_readable_obsidian_labels() {
        assert_eq!(display_label("obsidian_read"), "obs_read");
        assert_eq!(display_label("obsidian_list_vaults"), "obs_vault");
    }

    #[test]
    fn truncates_long_tool_names_with_marker() {
        assert_eq!(truncate_label("very_long_tool_name", 12), "very_long_t~");
    }

    #[test]
    fn formats_counts_with_commas() {
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_000), "1,000");
    }
}
