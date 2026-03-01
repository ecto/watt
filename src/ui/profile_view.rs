use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::profile::ProfileState;
use crate::ui::theme;

pub fn draw(f: &mut Frame, area: Rect, state: &ProfileState, scroll: &mut usize) {
    let (title, lines) = match state {
        ProfileState::Idle => (" Profile ", vec![Line::from("Press P to analyze system.")]),
        ProfileState::Loading => (
            " Profile — Analyzing... ",
            vec![Line::from(Span::styled(
                "Querying Claude...",
                theme::MUTED,
            ))],
        ),
        ProfileState::Ready(text) => {
            let lines: Vec<Line> = text.lines().map(|l| Line::from(l.to_string())).collect();
            (" Profile ", lines)
        }
        ProfileState::Error(msg) => (
            " Profile — Error ",
            vec![Line::from(Span::styled(
                msg.clone(),
                theme::ACCENT,
            ))],
        ),
    };

    let block = Block::default()
        .title(Span::styled(title, theme::TITLE))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::BORDER);

    // Clamp scroll to content
    let inner_height = area.height.saturating_sub(2) as usize; // borders
    let max_scroll = lines.len().saturating_sub(inner_height);
    if *scroll > max_scroll {
        *scroll = max_scroll;
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((*scroll as u16, 0));

    f.render_widget(paragraph, area);
}
