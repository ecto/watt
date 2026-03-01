#[derive(Clone, Debug)]
pub struct PowerSnapshot {
    /// Package power in watts (RAPL on Linux, stub elsewhere)
    pub package_watts: Option<f32>,
}

pub fn collect() -> PowerSnapshot {
    // TODO: RAPL on Linux via /sys/class/powercap
    PowerSnapshot {
        package_watts: None,
    }
}
