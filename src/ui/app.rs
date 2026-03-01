use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::collect::process::SortBy;
use crate::collect::SystemSnapshot;
use crate::ui::sparkline::RingBuffer;

const HISTORY_LEN: usize = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Chart,
    Processes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    Cpu,
    Mem,
    Gpu,
    Net,
    Disk,
    Pwr,
}

impl Metric {
    /// Returns a process sort key if this metric supports drill-down.
    pub fn to_sort_by(self) -> Option<SortBy> {
        match self {
            Metric::Cpu => Some(SortBy::Cpu),
            Metric::Mem => Some(SortBy::Memory),
            Metric::Gpu => Some(SortBy::Gpu),
            Metric::Net | Metric::Disk | Metric::Pwr => None,
        }
    }
}

pub struct App {
    pub should_quit: bool,

    // View state
    pub view: View,
    pub selected_metric: Metric,

    // Sparkline histories
    pub cpu_history: RingBuffer<f32>,
    pub mem_history: RingBuffer<f32>,
    pub gpu_history: RingBuffer<f32>,
    pub net_history: RingBuffer<f32>,
    pub disk_history: RingBuffer<f32>,
    pub pwr_history: RingBuffer<f32>,

    // Process table state
    pub sort_by: SortBy,
    pub proc_scroll: usize,
    pub filter_mode: bool,
    pub filter_text: String,

    // Latest snapshot
    pub snapshot: Option<SystemSnapshot>,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            view: View::Chart,
            selected_metric: Metric::Cpu,
            cpu_history: RingBuffer::new(HISTORY_LEN),
            mem_history: RingBuffer::new(HISTORY_LEN),
            gpu_history: RingBuffer::new(HISTORY_LEN),
            net_history: RingBuffer::new(HISTORY_LEN),
            disk_history: RingBuffer::new(HISTORY_LEN),
            pwr_history: RingBuffer::new(HISTORY_LEN),
            sort_by: SortBy::Cpu,
            proc_scroll: 0,
            filter_mode: false,
            filter_text: String::new(),
            snapshot: None,
        }
    }

    pub fn has_gpu(&self) -> bool {
        self.snapshot
            .as_ref()
            .map(|s| !s.gpus.is_empty())
            .unwrap_or(false)
    }

    fn has_net(&self) -> bool {
        self.snapshot
            .as_ref()
            .map(|s| s.network.total_bytes_sec() > 0.0)
            .unwrap_or(false)
            || self.net_history.as_vec().iter().any(|&v| v > 0.0)
    }

    fn has_disk(&self) -> bool {
        self.snapshot
            .as_ref()
            .map(|s| s.disk_io.total_bytes_sec() > 0.0)
            .unwrap_or(false)
            || self.disk_history.as_vec().iter().any(|&v| v > 0.0)
    }

    fn has_power(&self) -> bool {
        self.snapshot
            .as_ref()
            .and_then(|s| s.power.system_watts)
            .is_some()
    }

    /// Active metrics in display order, conditional on data availability.
    pub fn metrics(&self) -> Vec<Metric> {
        let mut v = vec![Metric::Cpu, Metric::Mem];
        if self.has_gpu() {
            v.push(Metric::Gpu);
        }
        if self.has_net() {
            v.push(Metric::Net);
        }
        if self.has_disk() {
            v.push(Metric::Disk);
        }
        if self.has_power() {
            v.push(Metric::Pwr);
        }
        v
    }

    fn metric_count(&self) -> usize {
        self.metrics().len()
    }

    fn metric_index(&self) -> usize {
        self.metrics()
            .iter()
            .position(|&m| m == self.selected_metric)
            .unwrap_or(0)
    }

    fn metric_from_index(&self, i: usize) -> Metric {
        let m = self.metrics();
        m.get(i).copied().unwrap_or(Metric::Cpu)
    }

    pub fn update(&mut self, snap: SystemSnapshot) {
        self.cpu_history.push(snap.cpu.aggregate);
        self.mem_history.push(snap.memory.ram_percent());
        if let Some(gpu) = snap.gpus.first() {
            self.gpu_history.push(gpu.utilization);
        }
        self.net_history.push(snap.network.total_bytes_sec());
        self.disk_history.push(snap.disk_io.total_bytes_sec());
        if let Some(w) = snap.power.system_watts {
            self.pwr_history.push(w);
        }
        self.snapshot = Some(snap);
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        // Quit from any view
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return;
            }
            _ => {}
        }

        match self.view {
            View::Chart => self.on_key_chart(key),
            View::Processes => self.on_key_processes(key),
        }
    }

    fn on_key_chart(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => {
                let i = self.metric_index();
                if i + 1 < self.metric_count() {
                    self.selected_metric = self.metric_from_index(i + 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.metric_index();
                if i > 0 {
                    self.selected_metric = self.metric_from_index(i - 1);
                }
            }
            KeyCode::Enter => {
                if let Some(sort) = self.selected_metric.to_sort_by() {
                    self.sort_by = sort;
                }
                self.proc_scroll = 0;
                self.view = View::Processes;
            }
            _ => {}
        }
    }

    fn on_key_processes(&mut self, key: KeyEvent) {
        if self.filter_mode {
            match key.code {
                KeyCode::Esc => {
                    self.filter_mode = false;
                    self.filter_text.clear();
                }
                KeyCode::Enter => {
                    self.filter_mode = false;
                }
                KeyCode::Backspace => {
                    self.filter_text.pop();
                }
                KeyCode::Char(c) => {
                    self.filter_text.push(c);
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.filter_text.clear();
                self.view = View::Chart;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.proc_scroll = self.proc_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.proc_scroll = self.proc_scroll.saturating_sub(1);
            }
            KeyCode::Char('g') => self.proc_scroll = 0,
            KeyCode::Char('G') => {
                if let Some(snap) = &self.snapshot {
                    self.proc_scroll = snap.processes.len().saturating_sub(1);
                }
            }
            KeyCode::Char('c') => self.sort_by = SortBy::Cpu,
            KeyCode::Char('m') => self.sort_by = SortBy::Memory,
            KeyCode::Char('p') => self.sort_by = SortBy::Pid,
            KeyCode::Char('n') => self.sort_by = SortBy::Name,
            KeyCode::Char('/') => {
                self.filter_mode = true;
                self.filter_text.clear();
            }
            _ => {}
        }
    }
}
