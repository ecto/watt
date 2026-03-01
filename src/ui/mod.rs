pub mod app;
pub mod cpu_view;
pub mod gpu_view;
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
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Gauge, Tabs};
use ratatui::Frame;
use ratatui::Terminal;

use crate::collect::memory::format_bytes;
use crate::collect::SystemSnapshot;
use crate::ui::app::{App, Tab};

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

    // Layout: header(1) + tabs(1) + body(rest)
    let outer = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(size);

    // Header
    header::draw(f, outer[0], snap);

    // Tabs bar
    draw_tabs(f, outer[1], app.tab);

    // Body
    match app.tab {
        Tab::Overview => draw_overview(f, outer[2], snap, app),
        Tab::Cpu => cpu_view::draw_detail(f, outer[2], &snap.cpu, &app.per_core_history),
        Tab::Gpu => gpu_view::draw(f, outer[2], &snap.gpus),
        Tab::Processes => process_view::draw(
            f,
            outer[2],
            &snap.processes,
            app.sort_by,
            app.proc_scroll,
            app.filter_mode,
            &app.filter_text,
        ),
    }
}

fn draw_tabs(f: &mut Frame, area: Rect, active: Tab) {
    let titles: Vec<Span> = Tab::ALL
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let num = i + 1;
            let style = if *t == active {
                theme::TAB_ACTIVE
            } else {
                theme::TAB_INACTIVE
            };
            Span::styled(format!(" {num}:{} ", t.label()), style)
        })
        .collect();

    let tabs = Tabs::new(titles)
        .select(active.index())
        .highlight_style(theme::TAB_ACTIVE.add_modifier(Modifier::UNDERLINED));
    f.render_widget(tabs, area);
}

fn draw_overview(f: &mut Frame, area: Rect, snap: &SystemSnapshot, app: &App) {
    // Split: CPU(40%) | right column [MEM + GPU + processes]
    let cols = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Left: CPU overview
    cpu_view::draw_overview(f, cols[0], &snap.cpu, &app.cpu_history);

    // Right: MEM gauge + GPU + process table
    let has_gpu = !snap.gpus.is_empty();
    let right_constraints = if has_gpu {
        vec![
            Constraint::Length(3),
            Constraint::Length(snap.gpus.len() as u16 * 3 + 2),
            Constraint::Min(8),
        ]
    } else {
        vec![Constraint::Length(3), Constraint::Min(8)]
    };
    let right = Layout::vertical(right_constraints).split(cols[1]);

    // Memory gauge
    draw_mem_gauge(f, right[0], snap);

    if has_gpu {
        gpu_view::draw(f, right[1], &snap.gpus);
        process_view::draw(
            f,
            right[2],
            &snap.processes,
            app.sort_by,
            app.proc_scroll,
            app.filter_mode,
            &app.filter_text,
        );
    } else {
        process_view::draw(
            f,
            right[1],
            &snap.processes,
            app.sort_by,
            app.proc_scroll,
            app.filter_mode,
            &app.filter_text,
        );
    }
}

fn draw_mem_gauge(f: &mut Frame, area: Rect, snap: &SystemSnapshot) {
    let pct = snap.memory.ram_percent();
    let block = Block::default()
        .title(Span::styled(" Memory ", theme::TITLE))
        .borders(Borders::ALL)
        .border_style(theme::BORDER);
    let gauge = Gauge::default()
        .block(block)
        .gauge_style(ratatui::style::Style::new().fg(theme::percent_color(pct)))
        .ratio((pct as f64 / 100.0).min(1.0))
        .label(format!(
            "{:.1}% ({}/{})",
            pct,
            format_bytes(snap.memory.used),
            format_bytes(snap.memory.total)
        ));
    f.render_widget(gauge, area);
}
