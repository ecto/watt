use sysinfo::System;

#[derive(Clone, Debug)]
pub struct CpuSnapshot {
    /// Per-core usage 0.0–100.0
    pub per_core: Vec<f32>,
    /// Aggregate usage 0.0–100.0
    pub aggregate: f32,
    /// CPU name (e.g. "Apple M1 Pro")
    pub name: String,
    /// Number of physical cores
    pub physical_cores: usize,
}

pub fn collect(sys: &System) -> CpuSnapshot {
    let cpus = sys.cpus();
    let per_core: Vec<f32> = cpus.iter().map(|c| c.cpu_usage()).collect();
    let aggregate = if per_core.is_empty() {
        0.0
    } else {
        per_core.iter().sum::<f32>() / per_core.len() as f32
    };
    let name = cpus.first().map(|c| c.brand().to_string()).unwrap_or_default();
    let physical_cores = sys.physical_core_count().unwrap_or(per_core.len());

    CpuSnapshot {
        per_core,
        aggregate,
        name,
        physical_cores,
    }
}
