//! Apple Silicon GPU backend via IOReport (utilization + power) and SMC (temperature).
//!
//! Based on macmon's proven approach — sudoless, low overhead, real-time metrics.

#![allow(non_upper_case_globals)]

use std::collections::HashMap;
use std::marker::{PhantomData, PhantomPinned};
use std::mem::{size_of, MaybeUninit};
use std::os::raw::c_void;
use std::ptr::null;
use std::time::Instant;

use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation::base::{
    kCFAllocatorDefault, kCFAllocatorNull, CFRange, CFRelease, CFTypeRef,
};
use core_foundation::data::{CFDataGetBytes, CFDataGetLength, CFDataRef};
use core_foundation::dictionary::{
    CFDictionaryCreateMutableCopy, CFDictionaryGetCount, CFDictionaryGetValue, CFDictionaryRef,
    CFMutableDictionaryRef,
};
use core_foundation::string::{
    kCFStringEncodingUTF8, CFStringCreateWithBytesNoCopy, CFStringGetCString, CFStringRef,
};

use super::gpu::{GpuBackend, GpuSnapshot};

type CVoidRef = *const c_void;

// ── CF helpers ──────────────────────────────────────────────────────

fn cfstr(val: &str) -> CFStringRef {
    unsafe {
        CFStringCreateWithBytesNoCopy(
            kCFAllocatorDefault,
            val.as_ptr(),
            val.len() as isize,
            kCFStringEncodingUTF8,
            false as u8,
            kCFAllocatorNull,
        )
    }
}

fn from_cfstr(val: CFStringRef) -> String {
    unsafe {
        let mut buf = vec![0i8; 256];
        if CFStringGetCString(val, buf.as_mut_ptr(), 256, kCFStringEncodingUTF8) == 0 {
            return String::new();
        }
        std::ffi::CStr::from_ptr(buf.as_ptr())
            .to_string_lossy()
            .to_string()
    }
}

fn cfdict_get_val(dict: CFDictionaryRef, key: &str) -> Option<CFTypeRef> {
    unsafe {
        let k = cfstr(key);
        let val = CFDictionaryGetValue(dict, k as _);
        CFRelease(k as _);
        if val.is_null() {
            None
        } else {
            Some(val)
        }
    }
}

fn safe_cfstr(ptr: CFStringRef) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        from_cfstr(ptr)
    }
}

// ── IOReport FFI ────────────────────────────────────────────────────

#[repr(C)]
struct IOReportSubscription {
    _data: [u8; 0],
    _phantom: PhantomData<(*mut u8, PhantomPinned)>,
}

type IOReportSubscriptionRef = *const IOReportSubscription;

#[link(name = "IOReport", kind = "dylib")]
unsafe extern "C" {
    fn IOReportCopyChannelsInGroup(
        a: CFStringRef,
        b: CFStringRef,
        c: u64,
        d: u64,
        e: u64,
    ) -> CFDictionaryRef;
    fn IOReportMergeChannels(a: CFDictionaryRef, b: CFDictionaryRef, nil: CFTypeRef);
    fn IOReportCreateSubscription(
        a: CVoidRef,
        b: CFMutableDictionaryRef,
        c: *mut CFMutableDictionaryRef,
        d: u64,
        e: CFTypeRef,
    ) -> IOReportSubscriptionRef;
    fn IOReportCreateSamples(
        a: IOReportSubscriptionRef,
        b: CFMutableDictionaryRef,
        c: CFTypeRef,
    ) -> CFDictionaryRef;
    fn IOReportCreateSamplesDelta(
        a: CFDictionaryRef,
        b: CFDictionaryRef,
        c: CFTypeRef,
    ) -> CFDictionaryRef;
    fn IOReportChannelGetGroup(a: CFDictionaryRef) -> CFStringRef;
    fn IOReportChannelGetSubGroup(a: CFDictionaryRef) -> CFStringRef;
    fn IOReportChannelGetChannelName(a: CFDictionaryRef) -> CFStringRef;
    fn IOReportSimpleGetIntegerValue(a: CFDictionaryRef, b: i32) -> i64;
    fn IOReportChannelGetUnitLabel(a: CFDictionaryRef) -> CFStringRef;
    fn IOReportStateGetCount(a: CFDictionaryRef) -> i32;
    fn IOReportStateGetNameForIndex(a: CFDictionaryRef, b: i32) -> CFStringRef;
    fn IOReportStateGetResidency(a: CFDictionaryRef, b: i32) -> i64;
}

use super::iokit_ffi::*;

// ── IOReport helpers ────────────────────────────────────────────────

fn cfio_get_residencies(item: CFDictionaryRef) -> Vec<(String, i64)> {
    let count = unsafe { IOReportStateGetCount(item) };
    (0..count)
        .map(|i| {
            let name = unsafe { IOReportStateGetNameForIndex(item, i) };
            let val = unsafe { IOReportStateGetResidency(item, i) };
            (from_cfstr(name), val)
        })
        .collect()
}

fn cfio_watts(item: CFDictionaryRef, unit: &str, duration_ms: u64) -> Option<f32> {
    let val = unsafe { IOReportSimpleGetIntegerValue(item, 0) } as f32;
    let per_sec = val / (duration_ms as f32 / 1000.0);
    match unit {
        "mJ" => Some(per_sec / 1e3),
        "uJ" => Some(per_sec / 1e6),
        "nJ" => Some(per_sec / 1e9),
        _ => None,
    }
}

struct IOReportIterItem {
    group: String,
    subgroup: String,
    channel: String,
    unit: String,
    item: CFDictionaryRef,
}

struct IOReportIter {
    sample: CFDictionaryRef,
    items: CFArrayRef,
    count: isize,
    index: isize,
}

impl IOReportIter {
    fn new(sample: CFDictionaryRef) -> Self {
        let items = cfdict_get_val(sample, "IOReportChannels").unwrap() as CFArrayRef;
        let count = unsafe { CFArrayGetCount(items) };
        Self {
            sample,
            items,
            count,
            index: 0,
        }
    }
}

impl Drop for IOReportIter {
    fn drop(&mut self) {
        unsafe { CFRelease(self.sample as _) };
    }
}

impl Iterator for IOReportIter {
    type Item = IOReportIterItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }
        let item =
            unsafe { CFArrayGetValueAtIndex(self.items, self.index) } as CFDictionaryRef;
        self.index += 1;

        Some(IOReportIterItem {
            group: safe_cfstr(unsafe { IOReportChannelGetGroup(item) }),
            subgroup: safe_cfstr(unsafe { IOReportChannelGetSubGroup(item) }),
            channel: safe_cfstr(unsafe { IOReportChannelGetChannelName(item) }),
            unit: safe_cfstr(unsafe { IOReportChannelGetUnitLabel(item) })
                .trim()
                .to_string(),
            item,
        })
    }
}

// ── SMC ─────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Default)]
struct KeyDataVer {
    major: u8,
    minor: u8,
    build: u8,
    reserved: u8,
    release: u16,
}

#[repr(C)]
#[derive(Debug, Default)]
struct PLimitData {
    version: u16,
    length: u16,
    cpu_p_limit: u32,
    gpu_p_limit: u32,
    mem_p_limit: u32,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
struct KeyInfo {
    data_size: u32,
    data_type: u32,
    data_attributes: u8,
}

#[repr(C)]
#[derive(Debug, Default)]
struct KeyData {
    key: u32,
    vers: KeyDataVer,
    p_limit_data: PLimitData,
    key_info: KeyInfo,
    result: u8,
    status: u8,
    data8: u8,
    data32: u32,
    bytes: [u8; 32],
}

struct SmcConn {
    conn: u32,
    key_cache: HashMap<u32, KeyInfo>,
}

impl SmcConn {
    fn open() -> Option<Self> {
        let mut conn = 0u32;
        let svc = std::ffi::CString::new("AppleSMC").ok()?;

        unsafe {
            let matching = IOServiceMatching(svc.as_ptr());
            let mut iter = 0u32;
            if IOServiceGetMatchingServices(0, matching, &mut iter) != 0 {
                return None;
            }

            let mut found = false;
            loop {
                let entry = IOIteratorNext(iter);
                if entry == 0 {
                    break;
                }
                let mut name = [0i8; 128];
                if IORegistryEntryGetName(entry, name.as_mut_ptr()) == 0 {
                    let entry_name =
                        std::ffi::CStr::from_ptr(name.as_ptr()).to_string_lossy();
                    if entry_name == "AppleSMCKeysEndpoint" {
                        let rs = IOServiceOpen(entry, mach_task_self(), 0, &mut conn);
                        IOObjectRelease(entry);
                        if rs != 0 {
                            IOObjectRelease(iter);
                            return None;
                        }
                        found = true;
                        break;
                    }
                }
                IOObjectRelease(entry);
            }
            IOObjectRelease(iter);

            if !found {
                return None;
            }
        }

        Some(Self {
            conn,
            key_cache: HashMap::new(),
        })
    }

    fn call(&self, input: &KeyData) -> Option<KeyData> {
        let mut output = KeyData::default();
        let mut osize = size_of::<KeyData>();
        let rs = unsafe {
            IOConnectCallStructMethod(
                self.conn,
                2,
                input as *const _ as _,
                size_of::<KeyData>(),
                &mut output as *mut _ as _,
                &mut osize,
            )
        };
        if rs != 0 || output.result != 0 {
            return None;
        }
        Some(output)
    }

    fn key_by_index(&self, index: u32) -> Option<String> {
        let input = KeyData {
            data8: 8,
            data32: index,
            ..Default::default()
        };
        let output = self.call(&input)?;
        std::str::from_utf8(&output.key.to_be_bytes())
            .ok()
            .map(|s| s.to_string())
    }

    fn read_key_info(&mut self, key: &str) -> Option<KeyInfo> {
        let key_u32 = key.bytes().fold(0u32, |acc, b| (acc << 8) + b as u32);
        if let Some(ki) = self.key_cache.get(&key_u32) {
            return Some(*ki);
        }
        let input = KeyData {
            data8: 9,
            key: key_u32,
            ..Default::default()
        };
        let output = self.call(&input)?;
        self.key_cache.insert(key_u32, output.key_info);
        Some(output.key_info)
    }

    fn read_val(&mut self, key: &str) -> Option<Vec<u8>> {
        let ki = self.read_key_info(key)?;
        let key_u32 = key.bytes().fold(0u32, |acc, b| (acc << 8) + b as u32);
        let input = KeyData {
            data8: 5,
            key: key_u32,
            key_info: ki,
            ..Default::default()
        };
        let output = self.call(&input)?;
        Some(output.bytes[..ki.data_size as usize].to_vec())
    }

    fn read_f32(&mut self, key: &str) -> Option<f32> {
        let data = self.read_val(key)?;
        if data.len() >= 4 {
            Some(f32::from_le_bytes(data[..4].try_into().ok()?))
        } else {
            None
        }
    }

    fn find_gpu_temp_keys(&mut self) -> Vec<String> {
        const FLOAT_TYPE: u32 = 1718383648; // "flt " as FourCC

        let count = match self.read_val("#KEY") {
            Some(d) if d.len() >= 4 => u32::from_be_bytes(d[..4].try_into().unwrap()),
            _ => return vec![],
        };

        let mut keys = Vec::new();
        for i in 0..count {
            let name = match self.key_by_index(i) {
                Some(n) => n,
                None => continue,
            };
            if !name.starts_with("Tg") {
                continue;
            }
            let ki = match self.read_key_info(&name) {
                Some(ki) => ki,
                None => continue,
            };
            if ki.data_size == 4 && ki.data_type == FLOAT_TYPE && self.read_val(&name).is_some() {
                keys.push(name);
            }
        }
        keys
    }
}

impl Drop for SmcConn {
    fn drop(&mut self) {
        unsafe {
            IOServiceClose(self.conn);
        }
    }
}

// ── SocInfo (GPU name + freq table) ─────────────────────────────────

fn get_gpu_info() -> Option<(String, Vec<u32>)> {
    let out = std::process::Command::new("system_profiler")
        .args(["SPHardwareDataType", "SPDisplaysDataType", "-json"])
        .output()
        .ok()?;

    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;

    let chip = json["SPHardwareDataType"][0]["chip_type"]
        .as_str()
        .unwrap_or("Apple GPU")
        .to_string();

    let gpu_cores = json["SPDisplaysDataType"][0]["sppci_cores"]
        .as_str()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);

    let name = if gpu_cores > 0 {
        format!("{chip} ({gpu_cores}-core GPU)")
    } else {
        chip
    };

    let freqs = get_gpu_freqs().unwrap_or_default();
    Some((name, freqs))
}

fn get_gpu_freqs() -> Option<Vec<u32>> {
    let svc = std::ffi::CString::new("AppleARMIODevice").ok()?;

    unsafe {
        let matching = IOServiceMatching(svc.as_ptr());
        let mut iter = 0u32;
        if IOServiceGetMatchingServices(0, matching, &mut iter) != 0 {
            return None;
        }

        let mut result = None;
        loop {
            let entry = IOIteratorNext(iter);
            if entry == 0 {
                break;
            }
            let mut name = [0i8; 128];
            if IORegistryEntryGetName(entry, name.as_mut_ptr()) != 0 {
                IOObjectRelease(entry);
                continue;
            }
            let entry_name = std::ffi::CStr::from_ptr(name.as_ptr()).to_string_lossy();
            if entry_name == "pmgr" {
                let mut props = MaybeUninit::<CFMutableDictionaryRef>::uninit();
                if IORegistryEntryCreateCFProperties(
                    entry,
                    props.as_mut_ptr(),
                    kCFAllocatorDefault,
                    0,
                ) == 0
                {
                    let dict = props.assume_init();
                    result = parse_dvfs_freqs(dict, "voltage-states9");
                    CFRelease(dict as _);
                }
                IOObjectRelease(entry);
                break;
            }
            IOObjectRelease(entry);
        }
        IOObjectRelease(iter);

        // Convert Hz → MHz
        result.map(|freqs| freqs.iter().map(|f| f / (1000 * 1000)).collect())
    }
}

fn parse_dvfs_freqs(dict: CFDictionaryRef, key: &str) -> Option<Vec<u32>> {
    let obj = cfdict_get_val(dict, key)? as CFDataRef;
    unsafe {
        let len = CFDataGetLength(obj);
        if len <= 0 {
            return None;
        }
        let mut buf = vec![0u8; len as usize];
        CFDataGetBytes(obj, CFRange::init(0, len), buf.as_mut_ptr());

        // Pairs of (freq_hz: u32, voltage: u32) — 8 bytes per entry
        let freqs: Vec<u32> = buf
            .chunks_exact(8)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        Some(freqs)
    }
}

// ── IOReport subscription ───────────────────────────────────────────

fn create_subscription() -> Option<(IOReportSubscriptionRef, CFMutableDictionaryRef)> {
    unsafe {
        let energy_group = cfstr("Energy Model");
        let gpu_group = cfstr("GPU Stats");
        let gpu_subgroup = cfstr("GPU Performance States");

        let chan1 = IOReportCopyChannelsInGroup(energy_group, null(), 0, 0, 0);
        let chan2 = IOReportCopyChannelsInGroup(gpu_group, gpu_subgroup, 0, 0, 0);

        CFRelease(energy_group as _);
        CFRelease(gpu_group as _);
        CFRelease(gpu_subgroup as _);

        if chan1.is_null() || chan2.is_null() {
            if !chan1.is_null() {
                CFRelease(chan1 as _);
            }
            if !chan2.is_null() {
                CFRelease(chan2 as _);
            }
            return None;
        }

        IOReportMergeChannels(chan1, chan2, null());

        let size = CFDictionaryGetCount(chan1);
        let chan = CFDictionaryCreateMutableCopy(kCFAllocatorDefault, size, chan1);
        CFRelease(chan1 as _);
        CFRelease(chan2 as _);

        if cfdict_get_val(chan, "IOReportChannels").is_none() {
            CFRelease(chan as _);
            return None;
        }

        let mut sub_out = MaybeUninit::<CFMutableDictionaryRef>::uninit();
        let sub = IOReportCreateSubscription(null(), chan, sub_out.as_mut_ptr(), 0, null());
        if sub.is_null() {
            CFRelease(chan as _);
            return None;
        }
        sub_out.assume_init();

        Some((sub, chan))
    }
}

// ── AppleGpuBackend ─────────────────────────────────────────────────

pub struct AppleGpuBackend {
    sub: IOReportSubscriptionRef,
    chan: CFMutableDictionaryRef,
    prev: Option<(CFDictionaryRef, Instant)>,
    smc: SmcConn,
    gpu_temp_keys: Vec<String>,
    gpu_name: String,
    #[allow(dead_code)]
    gpu_freqs: Vec<u32>,
    system_power_watts: Option<f32>,
}

// All fields are only accessed from the single collector thread.
unsafe impl Send for AppleGpuBackend {}

impl AppleGpuBackend {
    pub fn try_new() -> Option<Self> {
        let (sub, chan) = create_subscription()?;
        let mut smc = SmcConn::open()?;
        let gpu_temp_keys = smc.find_gpu_temp_keys();
        let (gpu_name, gpu_freqs) = get_gpu_info()?;

        // Take initial sample so the first collect() can compute a delta
        let initial = unsafe { IOReportCreateSamples(sub, chan, null()) };
        let prev = if initial.is_null() {
            None
        } else {
            Some((initial, Instant::now()))
        };

        Some(Self {
            sub,
            chan,
            prev,
            smc,
            gpu_temp_keys,
            gpu_name,
            gpu_freqs,
            system_power_watts: None,
        })
    }

    fn raw_sample(&self) -> Option<(CFDictionaryRef, Instant)> {
        let s = unsafe { IOReportCreateSamples(self.sub, self.chan, null()) };
        if s.is_null() {
            None
        } else {
            Some((s, Instant::now()))
        }
    }

    fn read_gpu_temp(&mut self) -> Option<f32> {
        if self.gpu_temp_keys.is_empty() {
            return None;
        }
        let temps: Vec<f32> = self
            .gpu_temp_keys
            .iter()
            .filter_map(|key| self.smc.read_f32(key))
            .filter(|&t| t > 0.0 && t < 150.0)
            .collect();
        if temps.is_empty() {
            None
        } else {
            Some(temps.iter().sum::<f32>() / temps.len() as f32)
        }
    }
}

impl GpuBackend for AppleGpuBackend {
    fn process_gpu_usage(&mut self) -> Vec<(u32, u64)> {
        vec![] // no public macOS API for per-process GPU memory
    }

    fn system_power_watts(&self) -> Option<f32> {
        self.system_power_watts
    }

    fn collect(&mut self) -> Vec<GpuSnapshot> {
        let cur = match self.raw_sample() {
            Some(s) => s,
            None => return vec![],
        };

        let prev = match self.prev.take() {
            Some(p) => p,
            None => {
                self.prev = Some(cur);
                return vec![];
            }
        };

        let dt_ms = cur.1.duration_since(prev.1).as_millis().max(1) as u64;

        let delta = unsafe { IOReportCreateSamplesDelta(prev.0, cur.0, null()) };
        unsafe { CFRelease(prev.0 as _) };
        self.prev = Some(cur);

        if delta.is_null() {
            return vec![];
        }

        // Safety: verify the channels array exists before creating the iterator
        if cfdict_get_val(delta, "IOReportChannels").is_none() {
            unsafe { CFRelease(delta as _) };
            return vec![];
        }

        let mut gpu_power = None;
        let mut system_power = 0.0f32;
        let mut has_energy = false;
        let mut gpu_util = 0.0f32;

        // IOReportIter takes ownership of delta and releases it on Drop
        for item in IOReportIter::new(delta) {
            if item.group == "Energy Model" {
                if let Some(w) = cfio_watts(item.item, &item.unit, dt_ms) {
                    system_power += w;
                    has_energy = true;
                    if item.channel == "GPU Energy" {
                        gpu_power = Some(w);
                    }
                }
            }

            if item.group == "GPU Stats"
                && item.subgroup == "GPU Performance States"
                && item.channel == "GPUPH"
            {
                let res = cfio_get_residencies(item.item);
                let offset = res
                    .iter()
                    .position(|r| r.0 != "IDLE" && r.0 != "OFF" && r.0 != "DOWN")
                    .unwrap_or(res.len());
                let total: f64 = res.iter().map(|r| r.1 as f64).sum();
                let active: f64 = res.iter().skip(offset).map(|r| r.1 as f64).sum();
                if total > 0.0 {
                    gpu_util = (active / total * 100.0) as f32;
                }
            }
        }

        self.system_power_watts = if has_energy { Some(system_power) } else { None };

        let temperature = self.read_gpu_temp();

        vec![GpuSnapshot {
            name: self.gpu_name.clone(),
            utilization: gpu_util,
            vram_used: 0,
            vram_total: 0,
            temperature,
            power_watts: gpu_power,
        }]
    }
}

impl Drop for AppleGpuBackend {
    fn drop(&mut self) {
        unsafe {
            if let Some((sample, _)) = self.prev.take() {
                CFRelease(sample as _);
            }
            CFRelease(self.chan as _);
            CFRelease(self.sub as _);
        }
    }
}
