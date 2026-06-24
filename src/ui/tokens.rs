use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, LineGauge, Paragraph},
};

use crate::data::session::SessionStats;

pub fn render(frame: &mut Frame, area: Rect, stats: &SessionStats) {
    let block = Block::default().title("TOKEN USAGE").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    let max = stats
        .tokens
        .total_tokens
        .max(stats.tokens.input)
        .max(stats.tokens.output)
        .max(stats.tokens.cache_read)
        .max(stats.tokens.cache_write)
        .max(1);

    render_line(
        frame,
        rows[0],
        "Input",
        stats.tokens.input,
        max,
        Color::Blue,
    );
    render_line(
        frame,
        rows[1],
        "Output",
        stats.tokens.output,
        max,
        Color::Green,
    );
    render_line(
        frame,
        rows[2],
        "Cache R",
        stats.tokens.cache_read,
        max,
        Color::Cyan,
    );
    render_line(
        frame,
        rows[3],
        "Cache W",
        stats.tokens.cache_write,
        max,
        Color::Magenta,
    );

    let cost = Paragraph::new(format!(
        "Cost ${:.4}  Total {}",
        stats.cost.total,
        format_count(stats.tokens.total_tokens)
    ));
    frame.render_widget(cost, rows[4]);
}

fn render_line(frame: &mut Frame, area: Rect, label: &str, value: u64, max: u64, color: Color) {
    let ratio = value as f64 / max as f64;
    let gauge = LineGauge::default()
        .label(format!("{label:<7} {}", format_count(value)))
        .ratio(ratio.clamp(0.0, 1.0))
        .filled_style(Style::default().fg(color));

    frame.render_widget(gauge, area);
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
