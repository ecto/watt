use ratatui::style::{Color, Modifier, Style};

pub const TITLE: Style = Style::new().fg(Color::White).add_modifier(Modifier::BOLD);
pub const LABEL: Style = Style::new().fg(Color::Gray);
pub const MUTED: Style = Style::new().fg(Color::DarkGray);
pub const HIGHLIGHT: Style = Style::new().fg(Color::Black).bg(Color::Cyan);
pub const BORDER: Style = Style::new().fg(Color::DarkGray);
pub const TAB_ACTIVE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
pub const TAB_INACTIVE: Style = Style::new().fg(Color::DarkGray);

/// Color for a percentage value: green < 60%, yellow < 85%, red >= 85%.
pub fn percent_color(pct: f32) -> Color {
    if pct >= 85.0 {
        Color::Red
    } else if pct >= 60.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

pub fn percent_style(pct: f32) -> Style {
    Style::new().fg(percent_color(pct))
}
