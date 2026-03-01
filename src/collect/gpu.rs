#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct GpuSnapshot {
    pub name: String,
    /// GPU utilization 0.0–100.0
    pub utilization: f32,
    /// VRAM used in bytes
    pub vram_used: u64,
    /// VRAM total in bytes
    pub vram_total: u64,
    /// Temperature in °C
    pub temperature: Option<f32>,
    /// Power draw in watts
    pub power_watts: Option<f32>,
}

impl GpuSnapshot {
    #[allow(dead_code)]
    pub fn vram_percent(&self) -> f32 {
        if self.vram_total == 0 {
            return 0.0;
        }
        self.vram_used as f32 / self.vram_total as f32 * 100.0
    }
}

pub trait GpuBackend: Send {
    fn collect(&mut self) -> Vec<GpuSnapshot>;
    /// Per-process GPU memory usage: vec of (pid, gpu_mem_bytes)
    fn process_gpu_usage(&mut self) -> Vec<(u32, u64)>;
    /// System-wide power draw in watts (if available)
    fn system_power_watts(&self) -> Option<f32> {
        None
    }
}

// --- NVIDIA backend (behind feature flag) ---

#[cfg(feature = "nvidia")]
pub struct NvidiaBackend {
    nvml: nvml_wrapper::Nvml,
}

#[cfg(feature = "nvidia")]
impl NvidiaBackend {
    pub fn try_new() -> Option<Self> {
        nvml_wrapper::Nvml::init().ok().map(|nvml| Self { nvml })
    }
}

#[cfg(feature = "nvidia")]
impl GpuBackend for NvidiaBackend {
    fn collect(&mut self) -> Vec<GpuSnapshot> {
        let count = match self.nvml.device_count() {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        (0..count)
            .filter_map(|i| {
                let dev = self.nvml.device_by_index(i).ok()?;
                let name = dev.name().unwrap_or_else(|_| format!("GPU {i}"));
                let util = dev.utilization_rates().ok().map(|u| u.gpu as f32).unwrap_or(0.0);
                let mem = dev.memory_info().ok();
                let temp = dev
                    .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                    .ok()
                    .map(|t| t as f32);
                let power = dev.power_usage().ok().map(|mw| mw as f32 / 1000.0);
                Some(GpuSnapshot {
                    name,
                    utilization: util,
                    vram_used: mem.as_ref().map(|m| m.used).unwrap_or(0),
                    vram_total: mem.as_ref().map(|m| m.total).unwrap_or(0),
                    temperature: temp,
                    power_watts: power,
                })
            })
            .collect()
    }

    fn process_gpu_usage(&mut self) -> Vec<(u32, u64)> {
        let count = match self.nvml.device_count() {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        let mut per_pid: std::collections::HashMap<u32, u64> = std::collections::HashMap::new();
        for i in 0..count {
            let dev = match self.nvml.device_by_index(i) {
                Ok(d) => d,
                Err(_) => continue,
            };
            if let Ok(procs) = dev.running_compute_processes() {
                for p in procs {
                    *per_pid.entry(p.pid).or_default() += p.used_gpu_memory.unwrap_or(0);
                }
            }
            if let Ok(procs) = dev.running_graphics_processes() {
                for p in procs {
                    *per_pid.entry(p.pid).or_default() += p.used_gpu_memory.unwrap_or(0);
                }
            }
        }
        per_pid.into_iter().collect()
    }
}

// --- NoGpu stub ---

pub struct NoGpu;

impl GpuBackend for NoGpu {
    fn collect(&mut self) -> Vec<GpuSnapshot> {
        vec![]
    }

    fn process_gpu_usage(&mut self) -> Vec<(u32, u64)> {
        vec![]
    }
}

pub fn default_backend() -> Box<dyn GpuBackend> {
    #[cfg(feature = "nvidia")]
    {
        if let Some(nv) = NvidiaBackend::try_new() {
            return Box::new(nv);
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(apple) = super::apple_gpu::AppleGpuBackend::try_new() {
            return Box::new(apple);
        }
    }
    Box::new(NoGpu)
}
