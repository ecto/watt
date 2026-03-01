use ratatui::layout::{Constraint, Rect};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;

use crate::collect::memory::format_bytes;
use crate::collect::process::{ProcessSnapshot, SortBy};
use crate::ui::theme;

pub fn draw(
    f: &mut Frame,
    area: Rect,
    procs: &[ProcessSnapshot],
    sort_by: SortBy,
    scroll: usize,
    filter_mode: bool,
    filter_text: &str,
) {
    let title = if filter_mode {
        format!(" Processes — filter: /{}_ ", filter_text)
    } else if !filter_text.is_empty() {
        format!(" Processes — filter: {} (Esc clear) ", filter_text)
    } else {
        format!(" Processes [sort: {}] ", sort_by.label())
    };

    let block = Block::default()
        .title(Span::styled(title, theme::TITLE))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::BORDER);

    // Filter
    let filtered: Vec<&ProcessSnapshot> = if filter_text.is_empty() {
        procs.iter().collect()
    } else {
        let f_lower = filter_text.to_lowercase();
        procs
            .iter()
            .filter(|p| p.name.to_lowercase().contains(&f_lower))
            .collect()
    };

    // Sort indicator in header
    let header_cells = [
        ("PID", SortBy::Pid),
        ("NAME", SortBy::Name),
        ("CPU%", SortBy::Cpu),
        ("MEM", SortBy::Memory),
        ("GPU", SortBy::Gpu),
        ("STATUS", SortBy::Cpu), // no sort for status
    ];

    let header = Row::new(header_cells.iter().map(|(label, col)| {
        let style = if *col == sort_by && *label != "STATUS" {
            theme::ACCENT
        } else {
            theme::LABEL
        };
        Cell::from(Span::styled(*label, style))
    }))
    .height(1);

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let cpu_style = theme::percent_style(p.cpu_percent);
            let gpu_text = if p.gpu_mem_bytes > 0 {
                format_bytes(p.gpu_mem_bytes)
            } else {
                "-".to_string()
            };
            let row = Row::new(vec![
                Cell::from(format!("{}", p.pid)),
                Cell::from(p.name.clone()),
                Cell::from(Span::styled(format!("{:.1}", p.cpu_percent), cpu_style)),
                Cell::from(format_bytes(p.memory_bytes)),
                Cell::from(gpu_text),
                Cell::from(Span::styled(&p.status, theme::MUTED)),
            ]);
            if i % 2 == 1 {
                row.style(theme::ALT_ROW)
            } else {
                row
            }
        })
        .collect();

    let widths = [
        Constraint::Length(7),
        Constraint::Min(20),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(8),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(theme::HIGHLIGHT);

    let mut state = TableState::default();
    if !filtered.is_empty() {
        state.select(Some(scroll.min(filtered.len().saturating_sub(1))));
    }

    f.render_stateful_widget(table, area, &mut state);
}
