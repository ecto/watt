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

    let constraints: Vec<Constraint> = gpus
        .iter()
        .map(|gpu| {
            if gpu.vram_total > 0 {
                Constraint::Length(3) // name + util + vram
            } else {
                Constraint::Length(2) // name + util
            }
        })
        .collect();
    let chunks = Layout::vertical(constraints).split(inner);

    for (i, gpu) in gpus.iter().enumerate() {
        if i >= chunks.len() {
            break;
        }
        let has_vram = gpu.vram_total > 0;
        let mut row_constraints = vec![Constraint::Length(1), Constraint::Length(1)];
        if has_vram {
            row_constraints.push(Constraint::Length(1));
        }
        let rows = Layout::vertical(row_constraints).split(chunks[i]);

        // Name + temp + power
        let mut info_spans = vec![Span::styled(&gpu.name, theme::TITLE)];
        if let Some(temp) = gpu.temperature {
            info_spans.push(Span::styled(format!("  {temp:.0}°C"), theme::percent_style(temp)));
        }
        if let Some(watts) = gpu.power_watts {
            info_spans.push(Span::styled(format!("  {watts:.1}W"), theme::LABEL));
        }
        f.render_widget(Line::from(info_spans), rows[0]);

        // Utilization gauge
        let util = gpu.utilization;
        let gauge = Gauge::default()
            .gauge_style(Style::new().fg(theme::percent_color(util)))
            .ratio((util as f64 / 100.0).min(1.0))
            .label(format!("util {util:.0}%"));
        f.render_widget(gauge, rows[1]);

        // VRAM gauge (only when meaningful)
        if has_vram {
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
}
