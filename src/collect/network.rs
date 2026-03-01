//! Network throughput collector using sysinfo::Networks.

use sysinfo::Networks;

#[derive(Clone, Debug, Default)]
pub struct NetworkSnapshot {
    pub rx_bytes_sec: f64,
    pub tx_bytes_sec: f64,
}

impl NetworkSnapshot {
    pub fn total_bytes_sec(&self) -> f32 {
        (self.rx_bytes_sec + self.tx_bytes_sec) as f32
    }
}

pub struct NetworkCollector {
    networks: Networks,
}

impl NetworkCollector {
    pub fn new() -> Self {
        let mut networks = Networks::new_with_refreshed_list();
        // Seed initial counters so first delta isn't cumulative
        networks.refresh();
        Self { networks }
    }

    /// Collect network throughput. `dt_secs` is the elapsed time since last call.
    pub fn collect(&mut self, dt_secs: f64) -> NetworkSnapshot {
        self.networks.refresh();

        let mut rx_total = 0u64;
        let mut tx_total = 0u64;

        for (_name, data) in &self.networks {
            rx_total += data.received();
            tx_total += data.transmitted();
        }

        if dt_secs <= 0.0 {
            return NetworkSnapshot::default();
        }

        NetworkSnapshot {
            rx_bytes_sec: rx_total as f64 / dt_secs,
            tx_bytes_sec: tx_total as f64 / dt_secs,
        }
    }
}
