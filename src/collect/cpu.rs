use sysinfo::System;

#[derive(Clone, Debug)]
pub struct CpuSnapshot {
    /// Per-core usage 0.0–100.0
    pub per_core: Vec<f32>,
    /// Aggregate usage 0.0–100.0
    pub aggregate: f32,
}

pub fn collect(sys: &System) -> CpuSnapshot {
    let cpus = sys.cpus();
    let per_core: Vec<f32> = cpus.iter().map(|c| c.cpu_usage()).collect();
    let aggregate = if per_core.is_empty() {
        0.0
    } else {
        per_core.iter().sum::<f32>() / per_core.len() as f32
    };
    CpuSnapshot {
        per_core,
        aggregate,
    }
}
