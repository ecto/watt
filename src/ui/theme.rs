use ratatui::style::{Color, Modifier, Style};

// Catppuccin Mocha palette
const SURFACE0: Color = Color::Rgb(49, 50, 68);
const SURFACE1: Color = Color::Rgb(69, 71, 90);
pub const TEXT: Color = Color::Rgb(205, 214, 244);
const SUBTEXT1: Color = Color::Rgb(166, 173, 200);
const SUBTEXT0: Color = Color::Rgb(108, 112, 134);
const SKY: Color = Color::Rgb(137, 220, 235);
const LAVENDER: Color = Color::Rgb(180, 190, 254);
const PEACH: Color = Color::Rgb(250, 179, 135);
// Gradient stops (used as tuples in percent_color)
// GREEN (166,227,161) → TEAL (148,226,213) → YELLOW (249,226,175) → PEACH → RED (243,139,168)

pub const TITLE: Style = Style::new().fg(LAVENDER).add_modifier(Modifier::BOLD);
pub const LABEL: Style = Style::new().fg(SUBTEXT1);
pub const MUTED: Style = Style::new().fg(SUBTEXT0);
pub const HIGHLIGHT: Style = Style::new().fg(SURFACE0).bg(SKY);
pub const BORDER: Style = Style::new().fg(SURFACE1);
pub const TAB_ACTIVE: Style = Style::new().fg(SKY).add_modifier(Modifier::BOLD);
pub const TAB_INACTIVE: Style = Style::new().fg(SUBTEXT0);
pub const ACCENT: Style = Style::new().fg(SKY).add_modifier(Modifier::BOLD);
pub const ALT_ROW: Style = Style::new().bg(SURFACE0);
pub const PEACH_STYLE: Style = Style::new().fg(PEACH).add_modifier(Modifier::BOLD);

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

fn lerp_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> Color {
    Color::Rgb(lerp_u8(a.0, b.0, t), lerp_u8(a.1, b.1, t), lerp_u8(a.2, b.2, t))
}

/// Smooth 5-stop gradient: GREEN → TEAL → YELLOW → PEACH → RED
pub fn percent_color(pct: f32) -> Color {
    let stops: [(f32, (u8, u8, u8)); 5] = [
        (0.0, (166, 227, 161)),   // GREEN
        (25.0, (148, 226, 213)),  // TEAL
        (50.0, (249, 226, 175)),  // YELLOW
        (75.0, (250, 179, 135)),  // PEACH
        (100.0, (243, 139, 168)), // RED
    ];
    let pct = pct.clamp(0.0, 100.0);
    for i in 0..stops.len() - 1 {
        let (lo, c0) = stops[i];
        let (hi, c1) = stops[i + 1];
        if pct <= hi {
            let t = (pct - lo) / (hi - lo);
            return lerp_color(c0, c1, t);
        }
    }
    let (_, (r, g, b)) = stops[stops.len() - 1];
    Color::Rgb(r, g, b)
}

pub fn percent_style(pct: f32) -> Style {
    Style::new().fg(percent_color(pct))
}
