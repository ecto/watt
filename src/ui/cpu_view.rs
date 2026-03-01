use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Gauge, Paragraph, Sparkline};
use ratatui::Frame;

use crate::ui::app::{App, Metric};
use crate::ui::sparkline::RingBuffer;
use crate::ui::theme;

/// Right-aligned sparkline data scaled to 0–1000 for percentage metrics.
fn sparkline_data(history: &RingBuffer<f32>, width: u16) -> Vec<u64> {
    let all: Vec<u64> = history.as_vec().iter().map(|&v| (v * 10.0) as u64).collect();
    let w = width as usize;
    if all.len() > w {
        all[all.len() - w..].to_vec()
    } else {
        let mut padded = vec![0u64; w - all.len()];
        padded.extend(all);
        padded
    }
}

/// Auto-scaled sparkline: scales to the peak value in the visible window.
/// Returns (data, peak_value). Sparkline max should be set to 1000.
fn sparkline_data_auto(history: &RingBuffer<f32>, width: u16) -> (Vec<u64>, f32) {
    let all = history.as_vec();
    let w = width as usize;

    let visible: &[f32] = if all.len() > w {
        &all[all.len() - w..]
    } else {
        &all
    };

    let peak = visible.iter().cloned().fold(0.0f32, f32::max).max(1.0);

    let data: Vec<u64> = if all.len() > w {
        all[all.len() - w..]
            .iter()
            .map(|&v| (v / peak * 1000.0) as u64)
            .collect()
    } else {
        let mut padded = vec![0u64; w - all.len()];
        padded.extend(all.iter().map(|&v| (v / peak * 1000.0) as u64));
        padded
    };

    (data, peak)
}

/// Format bytes/sec into human-readable rate.
pub fn format_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_000_000_000.0 {
        format!("{:.1}G/s", bytes_per_sec / 1_000_000_000.0)
    } else if bytes_per_sec >= 1_000_000.0 {
        format!("{:.1}M/s", bytes_per_sec / 1_000_000.0)
    } else if bytes_per_sec >= 1_000.0 {
        format!("{:.0}K/s", bytes_per_sec / 1_000.0)
    } else {
        format!("{:.0}B/s", bytes_per_sec)
    }
}

pub fn metric_color(metric: Metric) -> Color {
    match metric {
        Metric::Cpu => theme::MAUVE,
        Metric::Mem => theme::GREEN,
        Metric::Gpu => theme::TEAL,
        Metric::Net => theme::SKY,
        Metric::Disk => theme::LAVENDER,
        Metric::Pwr => theme::PEACH,
    }
}

/// Draw a percentage-based metric lane (CPU, MEM, GPU) with gauge + sparkline.
fn draw_metric(
    f: &mut Frame,
    area: Rect,
    title: String,
    pct: f32,
    history: &RingBuffer<f32>,
    metric: Metric,
    highlighted: bool,
) {
    let color = metric_color(metric);

    let (border_style, title_style) = if highlighted {
        (
            Style::new().fg(color),
            Style::new().fg(color).add_modifier(Modifier::BOLD),
        )
    } else {
        (Style::new().fg(theme::SURFACE1), theme::MUTED)
    };

    let block = Block::default()
        .title(Span::styled(format!(" {title} "), title_style))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 {
        return;
    }

    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner);

    let gauge = Gauge::default()
        .gauge_style(Style::new().fg(color))
        .ratio((pct as f64 / 100.0).min(1.0))
        .label(format!("{pct:.1}%"));
    f.render_widget(gauge, chunks[0]);

    if chunks[1].height > 0 {
        let data = sparkline_data(history, chunks[1].width);
        let spark = Sparkline::default()
            .data(&data)
            .max(1000)
            .style(Style::new().fg(color));
        f.render_widget(spark, chunks[1]);
    }
}

/// Draw a non-percentage metric lane (NET, DISK, PWR) with auto-scaled sparkline only.
fn draw_metric_auto(
    f: &mut Frame,
    area: Rect,
    title: String,
    history: &RingBuffer<f32>,
    metric: Metric,
    highlighted: bool,
) {
    let color = metric_color(metric);

    let (border_style, title_style) = if highlighted {
        (
            Style::new().fg(color),
            Style::new().fg(color).add_modifier(Modifier::BOLD),
        )
    } else {
        (Style::new().fg(theme::SURFACE1), theme::MUTED)
    };

    let block = Block::default()
        .title(Span::styled(format!(" {title} "), title_style))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 {
        return;
    }

    let (data, _peak) = sparkline_data_auto(history, inner.width);
    let spark = Sparkline::default()
        .data(&data)
        .max(1000)
        .style(Style::new().fg(color));
    f.render_widget(spark, inner);
}

/// Draw a compact sparkline strip inside a bordered block for the detail view.
pub fn draw_sparkline_strip(
    f: &mut Frame,
    area: Rect,
    title: &str,
    history: &RingBuffer<f32>,
    metric: Metric,
    is_percentage: bool,
) {
    let color = metric_color(metric);
    let block = Block::default()
        .title(Span::styled(
            format!(" {title} "),
            Style::new().fg(color).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 || inner.width < 1 {
        return;
    }

    if is_percentage {
        let data = sparkline_data(history, inner.width);
        let spark = Sparkline::default()
            .data(&data)
            .max(1000)
            .style(Style::new().fg(color));
        f.render_widget(spark, inner);
    } else {
        let (data, _) = sparkline_data_auto(history, inner.width);
        let spark = Sparkline::default()
            .data(&data)
            .max(1000)
            .style(Style::new().fg(color));
        f.render_widget(spark, inner);
    }
}

/// Number of terminal rows needed for the core heatmap (borders + data rows).
pub fn core_heatmap_height(core_count: usize) -> u16 {
    2 + ((core_count + 1) / 2) as u16
}

/// Draw a scrolling 2D heatmap: rows = core pairs, columns = time, color = utilization.
/// Uses Unicode half-block (▀) to pack 2 cores per terminal row via fg/bg coloring.
pub fn draw_core_heatmap(
    f: &mut Frame,
    area: Rect,
    per_core_history: &[RingBuffer<f32>],
    core_count: usize,
) {
    let block = Block::default()
        .title(Span::styled(
            " Cores ",
            Style::new().fg(theme::MAUVE).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(theme::MAUVE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 || inner.width < 1 || core_count == 0 {
        return;
    }

    let width = inner.width as usize;
    let row_pairs = (core_count + 1) / 2;

    let lines: Vec<Line> = (0..row_pairs)
        .map(|row| {
            let upper_idx = row * 2;
            let lower_idx = row * 2 + 1;
            let has_lower = lower_idx < core_count;

            let upper_data = per_core_history[upper_idx].as_vec();
            let lower_data = if has_lower {
                per_core_history[lower_idx].as_vec()
            } else {
                Vec::new()
            };

            let spans: Vec<Span> = (0..width)
                .map(|col| {
                    // Right-align: newest on right
                    let upper_val = if upper_data.len() > width {
                        upper_data[upper_data.len() - width + col]
                    } else if col >= width - upper_data.len() {
                        upper_data[col - (width - upper_data.len())]
                    } else {
                        0.0
                    };

                    let lower_val = if !has_lower {
                        -1.0 // sentinel for "no core"
                    } else if lower_data.len() > width {
                        lower_data[lower_data.len() - width + col]
                    } else if col >= width - lower_data.len() {
                        lower_data[col - (width - lower_data.len())]
                    } else {
                        0.0
                    };

                    let fg = theme::percent_color(upper_val);
                    let bg = if lower_val < 0.0 {
                        theme::SURFACE0
                    } else {
                        theme::percent_color(lower_val)
                    };

                    Span::styled("\u{2580}", Style::new().fg(fg).bg(bg))
                })
                .collect();

            Line::from(spans)
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

pub fn draw_chart(f: &mut Frame, area: Rect, app: &App) {
    let Some(snap) = &app.snapshot else {
        return;
    };

    let active = app.metrics();
    let n = active.len() as u32;
    if n == 0 {
        return;
    }

    let constraints: Vec<Constraint> = (0..n).map(|_| Constraint::Ratio(1, n)).collect();
    let rows = Layout::vertical(constraints).split(area);

    for (i, &metric) in active.iter().enumerate() {
        let highlighted = metric == app.selected_metric;
        let area = rows[i];

        match metric {
            Metric::Cpu => {
                let title = format!("CPU {:.1}%", snap.cpu.aggregate);
                draw_metric(f, area, title, snap.cpu.aggregate, &app.cpu_history, metric, highlighted);
            }
            Metric::Mem => {
                let pct = snap.memory.ram_percent();
                let title = format!("MEM {pct:.1}%");
                draw_metric(f, area, title, pct, &app.mem_history, metric, highlighted);
            }
            Metric::Gpu => {
                if let Some(gpu) = snap.gpus.first() {
                    let mut title = format!("GPU {:.1}%", gpu.utilization);
                    if let Some(temp) = gpu.temperature {
                        title.push_str(&format!(" {temp:.0}\u{00B0}C"));
                    }
                    draw_metric(f, area, title, gpu.utilization, &app.gpu_history, metric, highlighted);
                }
            }
            Metric::Net => {
                let title = format!(
                    "NET \u{2193}{} \u{2191}{}",
                    format_rate(snap.network.rx_bytes_sec),
                    format_rate(snap.network.tx_bytes_sec),
                );
                draw_metric_auto(f, area, title, &app.net_history, metric, highlighted);
            }
            Metric::Disk => {
                let title = format!(
                    "DISK R:{} W:{}",
                    format_rate(snap.disk_io.read_bytes_sec as f64),
                    format_rate(snap.disk_io.write_bytes_sec as f64),
                );
                draw_metric_auto(f, area, title, &app.disk_history, metric, highlighted);
            }
            Metric::Pwr => {
                if let Some(watts) = snap.power.system_watts {
                    let title = format!("PWR {watts:.1}W");
                    draw_metric_auto(f, area, title, &app.pwr_history, metric, highlighted);
                }
            }
        }
    }
}
