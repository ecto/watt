use std::collections::HashMap;

use sysinfo::System;

#[derive(Clone, Debug)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub gpu_mem_bytes: u64,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct ProcessGroup {
    pub name: String,
    pub count: usize,
    pub total_cpu: f32,
    pub total_memory: u64,
    pub total_gpu_mem: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortBy {
    Cpu,
    Memory,
    Gpu,
    Pid,
    Name,
}

impl SortBy {
    pub fn label(&self) -> &'static str {
        match self {
            SortBy::Cpu => "CPU%",
            SortBy::Memory => "MEM",
            SortBy::Gpu => "GPU",
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
                gpu_mem_bytes: 0,
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
        SortBy::Gpu => procs.sort_by(|a, b| b.gpu_mem_bytes.cmp(&a.gpu_mem_bytes)),
        SortBy::Pid => procs.sort_by(|a, b| a.pid.cmp(&b.pid)),
        SortBy::Name => procs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
    }

    procs
}

pub fn aggregate(procs: &[ProcessSnapshot]) -> Vec<ProcessGroup> {
    let mut map: HashMap<String, ProcessGroup> = HashMap::new();
    for p in procs {
        let entry = map.entry(p.name.clone()).or_insert(ProcessGroup {
            name: p.name.clone(),
            count: 0,
            total_cpu: 0.0,
            total_memory: 0,
            total_gpu_mem: 0,
        });
        entry.count += 1;
        entry.total_cpu += p.cpu_percent;
        entry.total_memory += p.memory_bytes;
        entry.total_gpu_mem += p.gpu_mem_bytes;
    }
    map.into_values().collect()
}

pub fn sort_groups(groups: &mut Vec<ProcessGroup>, sort_by: SortBy) {
    match sort_by {
        SortBy::Cpu => groups.sort_by(|a, b| {
            b.total_cpu
                .partial_cmp(&a.total_cpu)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        SortBy::Memory => groups.sort_by(|a, b| b.total_memory.cmp(&a.total_memory)),
        SortBy::Gpu => groups.sort_by(|a, b| b.total_gpu_mem.cmp(&a.total_gpu_mem)),
        SortBy::Pid | SortBy::Name => {
            groups.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        }
    }
}
