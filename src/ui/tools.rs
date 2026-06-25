use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::data::session::SessionStats;

const NAME_WIDTH: usize = 12;

pub fn render(frame: &mut Frame, area: Rect, stats: &SessionStats) {
    let visible_tool_rows = area.height.saturating_sub(4) as usize;
    let tools = visible_tools(stats, visible_tool_rows);
    let total_calls: u64 = stats.tool_counts.values().sum();
    let unique_tools = stats.tool_counts.len();

    let mut lines = vec![Line::from(vec![
        Span::raw("Total "),
        Span::styled(
            total_calls.to_string(),
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
    ])];

    if tools.is_empty() {
        lines.push(Line::from(Span::styled(
            "No tool calls",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.extend(tools.iter().map(|(name, count)| tool_line(name, *count)));
    }

    let paragraph =
        Paragraph::new(lines).block(Block::default().title("TOOL CALLS").borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}

fn tool_line<'a>(name: &'a str, count: u64) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{name:<NAME_WIDTH$}"),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format_count(count),
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

fn visible_tools(stats: &SessionStats, limit: usize) -> Vec<(String, u64)> {
    if limit == 0 {
        return Vec::new();
    }

    let mut tools: Vec<(String, u64)> = stats
        .tool_counts
        .iter()
        .map(|(name, count)| (display_label(name), *count))
        .collect();

    tools.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    if tools.len() <= limit {
        return tools;
    }

    let other_count: u64 = tools.iter().skip(limit - 1).map(|(_, count)| *count).sum();
    tools.truncate(limit - 1);
    tools.push(("Other".to_owned(), other_count));
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
    fn sorts_tools_by_count_descending() {
        let mut stats = SessionStats::default();
        stats.tool_counts.insert("read".to_owned(), 2);
        stats.tool_counts.insert("bash".to_owned(), 5);
        stats.tool_counts.insert("write".to_owned(), 1);

        let tools = visible_tools(&stats, 2);

        assert_eq!(tools, vec![("bash".to_owned(), 5), ("Other".to_owned(), 3)]);
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
