use sysinfo::System;

#[derive(Clone, Debug)]
pub struct MemorySnapshot {
    /// Used RAM in bytes
    pub used: u64,
    /// Total RAM in bytes
    pub total: u64,
}

impl MemorySnapshot {
    pub fn ram_percent(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        self.used as f32 / self.total as f32 * 100.0
    }

}

pub fn collect(sys: &System) -> MemorySnapshot {
    MemorySnapshot {
        used: sys.used_memory(),
        total: sys.total_memory(),
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;

    if bytes >= GIB {
        format!("{:.1}G", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.0}M", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.0}K", bytes as f64 / KIB as f64)
    } else {
        format!("{}B", bytes)
    }
}
