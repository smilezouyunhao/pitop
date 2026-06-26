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

pub fn dashboard(area: Rect, session_rows: usize) -> DashboardLayout {
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

    let session_height = adaptive_session_height(session_rows, vertical[3].height);
    let bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(session_height), Constraint::Min(3)])
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

fn adaptive_session_height(rows: usize, available_height: u16) -> u16 {
    let rows = rows.max(1) as u16;
    let desired = rows.saturating_add(3);
    let max = available_height.saturating_sub(3).max(4);

    desired.clamp(4, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_height_uses_one_base_row() {
        assert_eq!(adaptive_session_height(0, 20), 4);
        assert_eq!(adaptive_session_height(1, 20), 4);
        assert_eq!(adaptive_session_height(3, 20), 6);
    }

    #[test]
    fn session_height_leaves_space_for_logs() {
        assert_eq!(adaptive_session_height(20, 12), 9);
    }
}
