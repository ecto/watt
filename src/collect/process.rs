use sysinfo::System;

#[derive(Clone, Debug)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub status: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortBy {
    Cpu,
    Memory,
    Pid,
    Name,
}

impl SortBy {
    pub fn label(&self) -> &'static str {
        match self {
            SortBy::Cpu => "CPU%",
            SortBy::Memory => "MEM",
            SortBy::Pid => "PID",
            SortBy::Name => "NAME",
        }
    }
}

pub fn collect(sys: &System, sort_by: SortBy, filter: Option<&str>) -> Vec<ProcessSnapshot> {
    let mut procs: Vec<ProcessSnapshot> = sys
        .processes()
        .iter()
        .map(|(&pid, p)| {
            let pid_val: usize = pid.into();
            ProcessSnapshot {
                pid: pid_val as u32,
                name: p.name().to_string_lossy().to_string(),
                cpu_percent: p.cpu_usage(),
                memory_bytes: p.memory(),
                status: format!("{:?}", p.status()),
            }
        })
        .collect();

    if let Some(f) = filter {
        let f_lower = f.to_lowercase();
        procs.retain(|p| p.name.to_lowercase().contains(&f_lower));
    }

    match sort_by {
        SortBy::Cpu => procs.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal)),
        SortBy::Memory => procs.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes)),
        SortBy::Pid => procs.sort_by(|a, b| a.pid.cmp(&b.pid)),
        SortBy::Name => procs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
    }

    procs
}
