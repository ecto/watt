#[cfg(target_os = "macos")]
pub mod apple_gpu;
pub mod cpu;
pub mod gpu;
pub mod memory;
pub mod power;
pub mod process;

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use sysinfo::System;

use self::cpu::CpuSnapshot;
use self::gpu::GpuSnapshot;
use self::memory::MemorySnapshot;
use self::power::PowerSnapshot;
use self::process::{ProcessSnapshot, SortBy};

#[derive(Clone, Debug)]
pub struct SystemSnapshot {
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub processes: Vec<ProcessSnapshot>,
    pub gpus: Vec<GpuSnapshot>,
    pub power: PowerSnapshot,
    pub uptime: u64,
    pub hostname: String,
}

/// Spawn the collector on a dedicated OS thread.
/// Returns the receiver for snapshots and a handle to update sort/filter.
pub fn spawn_collector(
    interval: Duration,
) -> (mpsc::Receiver<SystemSnapshot>, thread::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        let mut sys = System::new_all();
        let mut gpu_backend = gpu::default_backend();

        // First refresh to seed deltas
        sys.refresh_all();
        thread::sleep(Duration::from_millis(250));

        loop {
            let start = Instant::now();

            sys.refresh_all();

            let snapshot = SystemSnapshot {
                cpu: cpu::collect(&sys),
                memory: memory::collect(&sys),
                processes: process::collect(&sys, SortBy::Cpu, None),
                gpus: gpu_backend.collect(),
                power: power::collect(),
                uptime: System::uptime(),
                hostname: System::host_name().unwrap_or_else(|| "unknown".into()),
            };

            if tx.send(snapshot).is_err() {
                break; // receiver dropped, exit
            }

            let elapsed = start.elapsed();
            if elapsed < interval {
                thread::sleep(interval - elapsed);
            }
        }
    });

    (rx, handle)
}
