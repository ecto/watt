use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::collect::process::SortBy;
use crate::collect::SystemSnapshot;
use crate::ui::sparkline::RingBuffer;

const HISTORY_LEN: usize = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Cpu,
    Gpu,
    Processes,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Overview, Tab::Cpu, Tab::Gpu, Tab::Processes];

    pub fn label(&self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Cpu => "CPU",
            Tab::Gpu => "GPU",
            Tab::Processes => "Processes",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Overview => 0,
            Tab::Cpu => 1,
            Tab::Gpu => 2,
            Tab::Processes => 3,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            0 => Tab::Overview,
            1 => Tab::Cpu,
            2 => Tab::Gpu,
            3 => Tab::Processes,
            _ => Tab::Overview,
        }
    }
}

pub struct App {
    pub tab: Tab,
    pub should_quit: bool,

    // Sparkline histories
    pub cpu_history: RingBuffer<f32>,
    pub per_core_history: Vec<RingBuffer<f32>>,
    pub mem_history: RingBuffer<f32>,

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
            tab: Tab::Overview,
            should_quit: false,
            cpu_history: RingBuffer::new(HISTORY_LEN),
            per_core_history: Vec::new(),
            mem_history: RingBuffer::new(HISTORY_LEN),
            sort_by: SortBy::Cpu,
            proc_scroll: 0,
            filter_mode: false,
            filter_text: String::new(),
            snapshot: None,
        }
    }

    pub fn update(&mut self, snap: SystemSnapshot) {
        // CPU aggregate
        self.cpu_history.push(snap.cpu.aggregate);

        // Per-core: grow history vec if needed
        while self.per_core_history.len() < snap.cpu.per_core.len() {
            self.per_core_history.push(RingBuffer::new(HISTORY_LEN));
        }
        for (i, &usage) in snap.cpu.per_core.iter().enumerate() {
            self.per_core_history[i].push(usage);
        }

        // Memory
        self.mem_history.push(snap.memory.ram_percent());

        self.snapshot = Some(snap);
    }

    pub fn on_key(&mut self, key: KeyEvent) {
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
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }

            // Tab switching
            KeyCode::Char('1') => self.tab = Tab::Overview,
            KeyCode::Char('2') => self.tab = Tab::Cpu,
            KeyCode::Char('3') => self.tab = Tab::Gpu,
            KeyCode::Char('4') => self.tab = Tab::Processes,
            KeyCode::Tab => {
                let next = (self.tab.index() + 1) % Tab::ALL.len();
                self.tab = Tab::from_index(next);
            }
            KeyCode::BackTab => {
                let prev = if self.tab.index() == 0 {
                    Tab::ALL.len() - 1
                } else {
                    self.tab.index() - 1
                };
                self.tab = Tab::from_index(prev);
            }

            // Vim scroll
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

            // Sort keys
            KeyCode::Char('c') => self.sort_by = SortBy::Cpu,
            KeyCode::Char('m') => self.sort_by = SortBy::Memory,
            KeyCode::Char('p') => self.sort_by = SortBy::Pid,
            KeyCode::Char('n') => self.sort_by = SortBy::Name,

            // Filter
            KeyCode::Char('/') => {
                self.filter_mode = true;
                self.filter_text.clear();
            }

            _ => {}
        }
    }
}
