use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::collect::memory::format_bytes;
use crate::collect::SystemSnapshot;
use crate::ui::theme;

pub fn draw(f: &mut Frame, area: Rect, snap: &SystemSnapshot) {
    let uptime_secs = snap.uptime;
    let hours = uptime_secs / 3600;
    let mins = (uptime_secs % 3600) / 60;

    let cpu_pct = snap.cpu.aggregate;
    let mem_pct = snap.memory.ram_percent();

    let line = Line::from(vec![
        Span::styled(&snap.hostname, theme::TITLE),
        Span::styled("  up ", theme::LABEL),
        Span::raw(format!("{hours}h{mins:02}m")),
        Span::styled("  cpu ", theme::LABEL),
        Span::styled(format!("{cpu_pct:.0}%"), theme::percent_style(cpu_pct)),
        Span::styled("  mem ", theme::LABEL),
        Span::styled(format!("{:.0}%", mem_pct), theme::percent_style(mem_pct)),
        Span::styled(
            format!(
                " ({}/{})",
                format_bytes(snap.memory.used),
                format_bytes(snap.memory.total)
            ),
            theme::MUTED,
        ),
        Span::styled(
            format!("  {} cores", snap.cpu.per_core.len()),
            theme::MUTED,
        ),
    ]);

    f.render_widget(line, area);
}
