use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardLayout {
    pub header: Rect,
    pub system: Rect,
    pub tokens: Rect,
    pub agent: Rect,
    pub tools: Rect,
    pub logs: Rect,
    pub footer: Rect,
}

pub fn dashboard(area: Rect) -> DashboardLayout {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(8),
            Constraint::Length(5),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(vertical[1]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(vertical[3]);

    DashboardLayout {
        header: vertical[0],
        system: top[0],
        tokens: top[1],
        agent: vertical[2],
        tools: bottom[0],
        logs: bottom[1],
        footer: vertical[4],
    }
}
