use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::data::session::SessionStats;

const METRIC_WIDTH: usize = 28;

pub fn render(frame: &mut Frame, area: Rect, stats: &SessionStats) {
    let lines = vec![
        paired_line(
            "Input",
            format_count(stats.tokens.input),
            Color::Blue,
            "Output",
            format_count(stats.tokens.output),
            Color::Green,
        ),
        single_line(
            "Cache R",
            format_count(stats.tokens.cache_read),
            Color::Cyan,
        ),
        paired_line(
            "Cost",
            format!("${:.4}", stats.cost.total),
            Color::Yellow,
            "Total",
            format_count(stats.tokens.total_tokens),
            Color::Magenta,
        ),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title("┤ TOKEN USAGE ├")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(paragraph, area);
}

fn single_line<'a>(label: &'a str, value: String, color: Color) -> Line<'a> {
    Line::from(metric_spans(label, value, color, None))
}

fn paired_line<'a>(
    left_label: &'a str,
    left_value: String,
    left_color: Color,
    right_label: &'a str,
    right_value: String,
    right_color: Color,
) -> Line<'a> {
    let left_width = metric_text_width(left_label, &left_value);
    let left_padding = METRIC_WIDTH.saturating_sub(left_width);

    let mut spans = metric_spans(left_label, left_value, left_color, Some(left_padding));
    spans.extend(metric_spans(right_label, right_value, right_color, None));
    Line::from(spans)
}

fn metric_spans<'a>(
    label: &'a str,
    value: String,
    color: Color,
    trailing_spaces: Option<usize>,
) -> Vec<Span<'a>> {
    vec![
        Span::styled(
            format!("{label:<8}"),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat(trailing_spaces.unwrap_or_default())),
    ]
}

fn metric_text_width(_label: &str, value: &str) -> usize {
    8 + value.chars().count()
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
    fn formats_counts_with_commas() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_000), "1,000");
        assert_eq!(format_count(1_234_567), "1,234,567");
    }
}
