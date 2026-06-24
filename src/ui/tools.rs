use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{BarChart, Block, Borders},
};

use crate::data::session::SessionStats;

pub fn render(frame: &mut Frame, area: Rect, stats: &SessionStats) {
    let labels_and_values = top_tools(stats, 8);
    let data: Vec<(&str, u64)> = labels_and_values
        .iter()
        .map(|(label, value)| (label.as_str(), *value))
        .collect();

    let chart = BarChart::default()
        .block(Block::default().title("TOOL CALLS").borders(Borders::ALL))
        .data(data.as_slice())
        .bar_width(7)
        .bar_gap(1)
        .bar_style(Style::default().fg(Color::LightBlue))
        .value_style(Style::default().fg(Color::White))
        .label_style(Style::default().fg(Color::Gray));

    frame.render_widget(chart, area);
}

fn top_tools(stats: &SessionStats, limit: usize) -> Vec<(String, u64)> {
    let mut tools: Vec<(String, u64)> = stats
        .tool_counts
        .iter()
        .map(|(name, count)| (short_label(name), *count))
        .collect();

    tools.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    tools.truncate(limit);
    tools
}

fn short_label(name: &str) -> String {
    name.chars().take(7).collect()
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

        let tools = top_tools(&stats, 2);

        assert_eq!(tools, vec![("bash".to_owned(), 5), ("read".to_owned(), 2)]);
    }
}
