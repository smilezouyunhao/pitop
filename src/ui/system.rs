use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Gauge},
};

use crate::data::sysinfo::SystemStats;

pub fn render(frame: &mut Frame, area: Rect, stats: &SystemStats) {
    let block = Block::default().title("SYSTEM").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(inner);

    render_gauge(
        frame,
        rows[0],
        "CPU",
        stats.cpu_usage_percent as f64,
        Color::Cyan,
    );
    render_gauge(
        frame,
        rows[1],
        "MEM",
        stats.memory_usage_percent(),
        Color::Green,
    );
    render_gauge(
        frame,
        rows[2],
        "DISK",
        stats.disk_usage_percent(),
        Color::Yellow,
    );
}

fn render_gauge(frame: &mut Frame, area: Rect, label: &str, percent: f64, color: Color) {
    let percent = percent.clamp(0.0, 100.0);
    let gauge = Gauge::default()
        .label(format!("{label} {percent:.0}%"))
        .ratio(percent / 100.0)
        .gauge_style(Style::default().fg(color));

    frame.render_widget(gauge, area);
}
