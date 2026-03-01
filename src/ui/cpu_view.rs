use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Gauge, Sparkline};
use ratatui::Frame;

use crate::collect::cpu::CpuSnapshot;
use crate::ui::sparkline::RingBuffer;
use crate::ui::theme;

pub fn draw_overview(f: &mut Frame, area: Rect, cpu: &CpuSnapshot, history: &RingBuffer<f32>) {
    let block = Block::default()
        .title(Span::styled(" CPU ", theme::TITLE))
        .borders(Borders::ALL)
        .border_style(theme::BORDER);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(inner);

    // Aggregate gauge
    let pct = cpu.aggregate;
    let gauge = Gauge::default()
        .gauge_style(Style::new().fg(theme::percent_color(pct)))
        .ratio((pct as f64 / 100.0).min(1.0))
        .label(format!("{:.1}% — {}", pct, cpu.name));
    f.render_widget(gauge, chunks[0]);

    // Sparkline
    let data: Vec<u64> = history.as_vec().iter().map(|&v| v as u64).collect();
    let spark = Sparkline::default()
        .data(&data)
        .max(100)
        .style(Style::new().fg(theme::percent_color(pct)));
    f.render_widget(spark, chunks[1]);
}

pub fn draw_detail(
    f: &mut Frame,
    area: Rect,
    cpu: &CpuSnapshot,
    per_core_history: &[RingBuffer<f32>],
) {
    let block = Block::default()
        .title(Span::styled(" CPU Cores ", theme::TITLE))
        .borders(Borders::ALL)
        .border_style(theme::BORDER);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let n_cores = cpu.per_core.len();
    if n_cores == 0 || inner.height == 0 {
        return;
    }

    // Two rows per core: gauge + sparkline
    let rows_per_core = 2u16;
    let max_visible = (inner.height / rows_per_core) as usize;
    let visible = n_cores.min(max_visible);

    let constraints: Vec<Constraint> = (0..visible)
        .map(|_| Constraint::Length(rows_per_core))
        .collect();
    let chunks = Layout::vertical(constraints).split(inner);

    for (i, chunk) in chunks.iter().enumerate() {
        if i >= n_cores {
            break;
        }
        let pct = cpu.per_core[i];
        let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(*chunk);

        let gauge = Gauge::default()
            .gauge_style(Style::new().fg(theme::percent_color(pct)))
            .ratio((pct as f64 / 100.0).min(1.0))
            .label(format!("core {i:>2}: {pct:.0}%"));
        f.render_widget(gauge, rows[0]);

        if i < per_core_history.len() {
            let data: Vec<u64> = per_core_history[i].as_vec().iter().map(|&v| v as u64).collect();
            let spark = Sparkline::default()
                .data(&data)
                .max(100)
                .style(Style::new().fg(theme::percent_color(pct)));
            f.render_widget(spark, rows[1]);
        }
    }
}
