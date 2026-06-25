use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardLayout {
    pub header: Rect,
    pub system: Rect,
    pub tokens: Rect,
    pub tools: Rect,
    pub session: Rect,
    pub logs: Rect,
}

pub fn dashboard(area: Rect) -> DashboardLayout {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(8),
            Constraint::Min(8),
        ])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(vertical[1]);

    let bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(vertical[3]);

    DashboardLayout {
        header: vertical[0],
        system: top[0],
        tokens: top[1],
        tools: vertical[2],
        session: bottom[0],
        logs: bottom[1],
    }
}
