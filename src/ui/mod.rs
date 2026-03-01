pub mod app;
pub mod cpu_view;
pub mod header;
pub mod process_view;
pub mod sparkline;
pub mod theme;

use std::io;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;
use ratatui::Terminal;

use crate::collect::process::SortBy;
use crate::ui::app::{App, View};

pub type Term = Terminal<CrosstermBackend<io::Stdout>>;

pub fn setup_terminal() -> anyhow::Result<Term> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    // Panic hook: restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    Ok(terminal)
}

pub fn restore_terminal(terminal: &mut Term) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let Some(snap) = &app.snapshot else {
        return;
    };

    let size = f.area();
    let outer = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(size);

    header::draw(f, outer[0], snap);

    match app.view {
        View::Chart => {
            // Populate layout cache for mouse hit-testing
            let active = app.metrics();
            let n = active.len() as u32;
            if n > 0 {
                let constraints: Vec<Constraint> =
                    (0..n).map(|_| Constraint::Ratio(1, n)).collect();
                let rows = Layout::vertical(constraints).split(outer[1]);
                app.layout.metric_rects = active
                    .iter()
                    .enumerate()
                    .map(|(i, &m)| (m, rows[i]))
                    .collect();
            } else {
                app.layout.metric_rects.clear();
            }
            app.layout.table_area = None;

            cpu_view::draw_chart(f, outer[1], app);
        }
        View::Processes => {
            app.layout.metric_rects.clear();

            let metric = app.selected_metric;
            let (history, is_pct): (&crate::ui::sparkline::RingBuffer<f32>, bool) = match metric {
                app::Metric::Cpu => (&app.cpu_history, true),
                app::Metric::Mem => (&app.mem_history, true),
                app::Metric::Gpu => (&app.gpu_history, true),
                app::Metric::Net => (&app.net_history, false),
                app::Metric::Disk => (&app.disk_history, false),
                app::Metric::Pwr => (&app.pwr_history, false),
            };

            let strip_title = match metric {
                app::Metric::Cpu => format!("CPU {:.1}%", snap.cpu.aggregate),
                app::Metric::Mem => format!("MEM {:.1}%", snap.memory.ram_percent()),
                app::Metric::Gpu => {
                    let mut t = String::from("GPU");
                    if let Some(gpu) = snap.gpus.first() {
                        t = format!("GPU {:.1}%", gpu.utilization);
                        if let Some(temp) = gpu.temperature {
                            t.push_str(&format!(" {temp:.0}\u{00B0}C"));
                        }
                    }
                    t
                }
                app::Metric::Net => format!(
                    "NET \u{2193}{} \u{2191}{}",
                    cpu_view::format_rate(snap.network.rx_bytes_sec),
                    cpu_view::format_rate(snap.network.tx_bytes_sec),
                ),
                app::Metric::Disk => format!(
                    "DISK R:{} W:{}",
                    cpu_view::format_rate(snap.disk_io.read_bytes_sec as f64),
                    cpu_view::format_rate(snap.disk_io.write_bytes_sec as f64),
                ),
                app::Metric::Pwr => {
                    if let Some(w) = snap.power.system_watts {
                        format!("PWR {w:.1}W")
                    } else {
                        "PWR --".into()
                    }
                }
            };

            // CPU detail: heatmap + sparkline + process table
            let show_heatmap = metric == app::Metric::Cpu && !app.per_core_history.is_empty();

            let table_rect;
            if show_heatmap {
                let hm_h = cpu_view::core_heatmap_height(app.per_core_history.len());
                let detail = Layout::vertical([
                    Constraint::Length(hm_h),
                    Constraint::Length(5),
                    Constraint::Min(0),
                ])
                .split(outer[1]);

                cpu_view::draw_core_heatmap(
                    f,
                    detail[0],
                    &app.per_core_history,
                    app.per_core_history.len(),
                );
                cpu_view::draw_sparkline_strip(f, detail[1], &strip_title, history, metric, is_pct);
                table_rect = detail[2];
                process_view::draw(
                    f,
                    detail[2],
                    &snap.processes,
                    app.sort_by,
                    app.proc_scroll,
                    app.filter_mode,
                    &app.filter_text,
                );
            } else {
                let detail = Layout::vertical([Constraint::Length(5), Constraint::Min(0)])
                    .split(outer[1]);

                cpu_view::draw_sparkline_strip(f, detail[0], &strip_title, history, metric, is_pct);
                table_rect = detail[1];
                process_view::draw(
                    f,
                    detail[1],
                    &snap.processes,
                    app.sort_by,
                    app.proc_scroll,
                    app.filter_mode,
                    &app.filter_text,
                );
            }

            // Populate table layout cache for mouse hit-testing
            // Table has a border (1px each side), then header row, then data rows
            app.layout.table_area = Some(table_rect);
            let inner_x = table_rect.x + 1; // border
            let header_y = table_rect.y + 1; // border
            app.layout.table_header_y = Some(header_y);
            app.layout.table_first_row_y = Some(header_y + 1);

            // Compute column x-ranges from the same widths used in process_view
            let col_defs: [(SortBy, u16); 6] = [
                (SortBy::Pid, 7),
                (SortBy::Name, 20), // Min(20) — approximate
                (SortBy::Cpu, 8),
                (SortBy::Memory, 8),
                (SortBy::Gpu, 8),
                (SortBy::Cpu, 8), // STATUS column (no distinct sort)
            ];
            let table_inner_width = table_rect.width.saturating_sub(2);
            // Fixed columns total
            let fixed: u16 = 7 + 8 + 8 + 8 + 8; // 39
            let name_width = table_inner_width.saturating_sub(fixed).max(20);
            let actual_widths: [u16; 6] = [7, name_width, 8, 8, 8, 8];

            let mut col_ranges = Vec::with_capacity(5); // skip STATUS
            let mut cx = inner_x;
            for (i, &(sort_key, _)) in col_defs.iter().enumerate() {
                let w = actual_widths[i];
                // Skip the STATUS column (index 5) — it doesn't have a unique sort
                if i < 5 {
                    col_ranges.push((sort_key, cx, cx + w));
                }
                cx += w + 1; // +1 for column gap
            }
            app.layout.col_ranges = col_ranges;
        }
    }
}
