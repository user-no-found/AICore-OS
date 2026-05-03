use ratatui::style::{Color, Modifier, Style};

pub fn surface_style() -> Style {
    Style::default()
        .fg(Color::Rgb(224, 229, 222))
        .bg(Color::Rgb(9, 13, 14))
}

pub fn normal_style() -> Style {
    Style::default().fg(Color::Rgb(224, 229, 222))
}

pub fn title_style() -> Style {
    Style::default()
        .fg(Color::Rgb(248, 229, 170))
        .add_modifier(Modifier::BOLD)
}

pub fn focus_style() -> Style {
    Style::default().fg(Color::Rgb(127, 205, 184))
}

pub fn dim_style() -> Style {
    Style::default().fg(Color::Rgb(113, 123, 119))
}

pub fn prompt_style() -> Style {
    Style::default().fg(Color::Rgb(147, 197, 253))
}

pub fn assistant_style() -> Style {
    Style::default().fg(Color::Rgb(127, 205, 184))
}

pub fn tool_style() -> Style {
    Style::default().fg(Color::Rgb(250, 204, 116))
}

pub fn approval_style() -> Style {
    Style::default().fg(Color::Rgb(251, 146, 60))
}

pub fn terminal_style() -> Style {
    Style::default().fg(Color::Rgb(139, 148, 158))
}

pub fn code_style() -> Style {
    Style::default().fg(Color::Rgb(190, 242, 100))
}

pub fn diff_style() -> Style {
    Style::default().fg(Color::Rgb(216, 180, 254))
}

pub fn diff_add_style() -> Style {
    Style::default().fg(Color::Rgb(134, 239, 172))
}

pub fn diff_remove_style() -> Style {
    Style::default().fg(Color::Rgb(252, 165, 165))
}

pub fn media_style() -> Style {
    Style::default().fg(Color::Rgb(103, 232, 249))
}
