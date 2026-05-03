use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct UiAreas {
    pub top: Rect,
    pub left: Rect,
    pub center: Rect,
    pub right: Rect,
    pub composer: Rect,
    pub action: Rect,
}

impl UiAreas {
    pub fn new(area: Rect) -> Self {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(12),
                Constraint::Length(5),
                Constraint::Length(2),
            ])
            .split(area);
        let main = if area.width >= 140 {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(32),
                    Constraint::Min(60),
                    Constraint::Length(34),
                ])
                .split(vertical[1])
        } else if area.width >= 100 {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(28),
                    Constraint::Min(60),
                    Constraint::Length(0),
                ])
                .split(vertical[1])
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(0),
                    Constraint::Min(40),
                    Constraint::Length(0),
                ])
                .split(vertical[1])
        };

        Self {
            top: vertical[0],
            left: main[0],
            center: main[1],
            right: main[2],
            composer: vertical[2],
            action: vertical[3],
        }
    }
}
