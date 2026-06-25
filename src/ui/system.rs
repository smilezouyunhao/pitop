use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::data::sysinfo::SystemStats;

const BAR_WIDTH: usize = 10;

pub fn render(frame: &mut Frame, area: Rect, stats: &SystemStats) {
    let lines = vec![
        metric_line("CPU", stats.cpu_usage_percent as f64, None),
        metric_line(
            "MEM",
            stats.memory_usage_percent(),
            Some(format!(
                "{}/{}",
                format_bytes(stats.memory_used_bytes),
                format_bytes(stats.memory_total_bytes)
            )),
        ),
        metric_line(
            "DISK",
            stats.disk_usage_percent(),
            Some(format!(
                "{}/{}",
                format_bytes(stats.disk_used_bytes),
                format_bytes(stats.disk_total_bytes)
            )),
        ),
    ];

    let paragraph =
        Paragraph::new(lines).block(Block::default().title("SYSTEM").borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}

fn metric_line<'a>(label: &'a str, percent: f64, suffix: Option<String>) -> Line<'a> {
    let percent = percent.clamp(0.0, 100.0);
    let color = usage_color(percent);
    let suffix = suffix.unwrap_or_default();

    Line::from(vec![
        Span::styled(
            format!("{label:<4}"),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("{:>3.0}%  ", percent), Style::default().fg(color)),
        Span::styled(usage_bar(percent, BAR_WIDTH), Style::default().fg(color)),
        Span::raw(format!("  {suffix}")),
    ])
}

fn usage_color(percent: f64) -> Color {
    if percent >= 80.0 {
        Color::Red
    } else if percent >= 60.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn usage_bar(percent: f64, width: usize) -> String {
    let filled = ((percent.clamp(0.0, 100.0) / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "▰".repeat(filled), "▱".repeat(empty))
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];

    let mut value = bytes as f64;
    let mut unit = UNITS[0];
    for next_unit in UNITS.iter().skip(1) {
        if value < 1024.0 {
            break;
        }
        value /= 1024.0;
        unit = next_unit;
    }

    if unit == "B" {
        format!("{bytes}B")
    } else {
        format!("{value:.1}{unit}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_usage_bar() {
        assert_eq!(usage_bar(0.0, 4), "▱▱▱▱");
        assert_eq!(usage_bar(50.0, 4), "▰▰▱▱");
        assert_eq!(usage_bar(100.0, 4), "▰▰▰▰");
    }

    #[test]
    fn formats_bytes() {
        assert_eq!(format_bytes(512), "512B");
        assert_eq!(format_bytes(1024), "1.0K");
        assert_eq!(format_bytes(16 * 1024 * 1024 * 1024), "16.0G");
    }

    #[test]
    fn chooses_usage_color_by_threshold() {
        assert_eq!(usage_color(59.9), Color::Green);
        assert_eq!(usage_color(60.0), Color::Yellow);
        assert_eq!(usage_color(80.0), Color::Red);
    }
}
