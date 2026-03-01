#[cfg(target_os = "macos")]
pub mod apple_gpu;
pub mod cpu;
pub mod disk;
pub mod gpu;
#[cfg(target_os = "macos")]
pub mod iokit_ffi;
pub mod memory;
pub mod network;
pub mod power;
pub mod process;

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use sysinfo::System;

use self::cpu::CpuSnapshot;
use self::disk::{DiskIoCollector, DiskIoSnapshot};
use self::gpu::GpuSnapshot;
use self::memory::MemorySnapshot;
use self::network::{NetworkCollector, NetworkSnapshot};
use self::power::PowerSnapshot;
use self::process::{ProcessSnapshot, SortBy};

#[derive(Clone, Debug)]
pub struct SystemSnapshot {
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub processes: Vec<ProcessSnapshot>,
    pub gpus: Vec<GpuSnapshot>,
    pub network: NetworkSnapshot,
    pub disk_io: DiskIoSnapshot,
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
        let mut net_collector = NetworkCollector::new();
        let mut disk_collector = DiskIoCollector::new();

        // First refresh to seed deltas
        sys.refresh_all();
        thread::sleep(Duration::from_millis(250));

        let mut last_tick = Instant::now();

        loop {
            let start = Instant::now();
            let dt_secs = start.duration_since(last_tick).as_secs_f64();
            last_tick = start;

            sys.refresh_all();

            let gpus = gpu_backend.collect();
            let system_watts = gpu_backend.system_power_watts();
            let gpu_per_process = gpu_backend.process_gpu_usage();
            let mut processes = process::collect(&sys, SortBy::Cpu, None);

            // Merge per-process GPU memory into snapshots
            for proc in &mut processes {
                for &(pid, mem) in &gpu_per_process {
                    if proc.pid == pid {
                        proc.gpu_mem_bytes = mem;
                        break;
                    }
                }
            }

            let network = net_collector.collect(dt_secs);
            let disk_io = disk_collector.collect(dt_secs);

            let snapshot = SystemSnapshot {
                cpu: cpu::collect(&sys),
                memory: memory::collect(&sys),
                processes,
                gpus,
                network,
                disk_io,
                power: PowerSnapshot { system_watts },
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
