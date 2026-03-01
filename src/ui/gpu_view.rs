use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui::Frame;

use crate::collect::gpu::GpuSnapshot;
use crate::collect::memory::format_bytes;
use crate::ui::theme;

pub fn draw(f: &mut Frame, area: Rect, gpus: &[GpuSnapshot]) {
    let block = Block::default()
        .title(Span::styled(" GPU ", theme::TITLE))
        .borders(Borders::ALL)
        .border_style(theme::BORDER);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if gpus.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "No GPU detected (enable --features nvidia for NVIDIA)",
            theme::MUTED,
        )));
        f.render_widget(msg, inner);
        return;
    }

    // 3 rows per GPU: name, util gauge, vram gauge
    let constraints: Vec<Constraint> = gpus.iter().map(|_| Constraint::Length(3)).collect();
    let chunks = Layout::vertical(constraints).split(inner);

    for (i, gpu) in gpus.iter().enumerate() {
        if i >= chunks.len() {
            break;
        }
        let rows =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
                .split(chunks[i]);

        // Name + temp + power
        let mut info_spans = vec![Span::styled(&gpu.name, theme::TITLE)];
        if let Some(temp) = gpu.temperature {
            info_spans.push(Span::styled(format!("  {temp:.0}°C"), theme::percent_style(temp)));
        }
        if let Some(watts) = gpu.power_watts {
            info_spans.push(Span::styled(format!("  {watts:.0}W"), theme::LABEL));
        }
        f.render_widget(Line::from(info_spans), rows[0]);

        // Utilization gauge
        let util = gpu.utilization;
        let gauge = Gauge::default()
            .gauge_style(Style::new().fg(theme::percent_color(util)))
            .ratio((util as f64 / 100.0).min(1.0))
            .label(format!("util {util:.0}%"));
        f.render_widget(gauge, rows[1]);

        // VRAM gauge
        let vram_pct = gpu.vram_percent();
        let vram_gauge = Gauge::default()
            .gauge_style(Style::new().fg(theme::percent_color(vram_pct)))
            .ratio((vram_pct as f64 / 100.0).min(1.0))
            .label(format!(
                "vram {:.0}% ({}/{})",
                vram_pct,
                format_bytes(gpu.vram_used),
                format_bytes(gpu.vram_total)
            ));
        f.render_widget(vram_gauge, rows[2]);
    }
}
