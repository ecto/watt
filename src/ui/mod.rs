pub mod app;
pub mod cpu_view;
pub mod header;
pub mod process_view;
pub mod sparkline;
pub mod theme;

use std::io;

use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;
use ratatui::Terminal;

use crate::ui::app::{App, View};

pub type Term = Terminal<CrosstermBackend<io::Stdout>>;

pub fn setup_terminal() -> anyhow::Result<Term> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    // Panic hook: restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    Ok(terminal)
}

pub fn restore_terminal(terminal: &mut Term) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

pub fn draw(f: &mut Frame, app: &App) {
    let Some(snap) = &app.snapshot else {
        return;
    };

    let size = f.area();
    let outer = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(size);

    header::draw(f, outer[0], snap);

    match app.view {
        View::Chart => {
            cpu_view::draw_chart(f, outer[1], app);
        }
        View::Processes => {
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

            let detail = Layout::vertical([Constraint::Length(5), Constraint::Min(0)])
                .split(outer[1]);

            cpu_view::draw_sparkline_strip(f, detail[0], &strip_title, history, metric, is_pct);

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
    }
}
