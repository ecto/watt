//! Disk I/O throughput collector.
//!
//! macOS: reads cumulative byte counters from IOKit's IOBlockStorageDriver.
//! Other platforms: stub returning zeros.

#[derive(Clone, Debug, Default)]
pub struct DiskIoSnapshot {
    pub read_bytes_sec: f32,
    pub write_bytes_sec: f32,
}

impl DiskIoSnapshot {
    pub fn total_bytes_sec(&self) -> f32 {
        self.read_bytes_sec + self.write_bytes_sec
    }
}

// ── macOS implementation ────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod platform {
    use super::DiskIoSnapshot;
    use crate::collect::iokit_ffi::*;
    use core_foundation::base::{kCFAllocatorDefault, CFRelease, CFTypeRef};
    use core_foundation::dictionary::{CFDictionaryGetValue, CFDictionaryRef, CFMutableDictionaryRef};
    use core_foundation::number::CFNumberRef;
    use core_foundation::string::{kCFStringEncodingUTF8, CFStringCreateWithCString};
    use std::mem::MaybeUninit;

    fn cf_dict_get_i64(dict: CFDictionaryRef, key: &str) -> Option<i64> {
        unsafe {
            let ckey = std::ffi::CString::new(key).ok()?;
            let cfkey = CFStringCreateWithCString(
                kCFAllocatorDefault,
                ckey.as_ptr(),
                kCFStringEncodingUTF8,
            );
            if cfkey.is_null() {
                return None;
            }
            let val = CFDictionaryGetValue(dict, cfkey as CFTypeRef);
            CFRelease(cfkey as CFTypeRef);
            if val.is_null() {
                return None;
            }
            let mut out: i64 = 0;
            if core_foundation::number::CFNumberGetValue(
                val as CFNumberRef,
                core_foundation::number::kCFNumberSInt64Type,
                &mut out as *mut i64 as *mut std::ffi::c_void,
            ) {
                Some(out)
            } else {
                None
            }
        }
    }

    fn read_disk_counters() -> Option<(i64, i64)> {
        let class = std::ffi::CString::new("IOBlockStorageDriver").ok()?;
        unsafe {
            let matching = IOServiceMatching(class.as_ptr());
            if matching.is_null() {
                return None;
            }
            let mut iter = 0u32;
            if IOServiceGetMatchingServices(0, matching, &mut iter) != 0 {
                return None;
            }

            let mut total_read = 0i64;
            let mut total_write = 0i64;

            loop {
                let entry = IOIteratorNext(iter);
                if entry == 0 {
                    break;
                }

                let mut props = MaybeUninit::<CFMutableDictionaryRef>::uninit();
                if IORegistryEntryCreateCFProperties(
                    entry,
                    props.as_mut_ptr(),
                    kCFAllocatorDefault,
                    0,
                ) == 0
                {
                    let dict = props.assume_init();
                    // Look for "Statistics" sub-dictionary
                    let stats_key = std::ffi::CString::new("Statistics").unwrap();
                    let cfkey = CFStringCreateWithCString(
                        kCFAllocatorDefault,
                        stats_key.as_ptr(),
                        kCFStringEncodingUTF8,
                    );
                    if !cfkey.is_null() {
                        let stats_val = CFDictionaryGetValue(dict as CFDictionaryRef, cfkey as CFTypeRef);
                        CFRelease(cfkey as CFTypeRef);
                        if !stats_val.is_null() {
                            let stats = stats_val as CFDictionaryRef;
                            if let Some(r) = cf_dict_get_i64(stats, "Bytes (Read)") {
                                total_read += r;
                            }
                            if let Some(w) = cf_dict_get_i64(stats, "Bytes (Write)") {
                                total_write += w;
                            }
                        }
                    }
                    CFRelease(dict as CFTypeRef);
                }

                IOObjectRelease(entry);
            }
            IOObjectRelease(iter);

            Some((total_read, total_write))
        }
    }

    pub struct DiskIoCollector {
        prev_read: i64,
        prev_write: i64,
    }

    impl DiskIoCollector {
        pub fn new() -> Self {
            let (r, w) = read_disk_counters().unwrap_or((0, 0));
            Self {
                prev_read: r,
                prev_write: w,
            }
        }

        pub fn collect(&mut self, dt_secs: f64) -> DiskIoSnapshot {
            let (cur_read, cur_write) = match read_disk_counters() {
                Some(c) => c,
                None => return DiskIoSnapshot::default(),
            };

            let dr = (cur_read - self.prev_read).max(0) as f64;
            let dw = (cur_write - self.prev_write).max(0) as f64;
            self.prev_read = cur_read;
            self.prev_write = cur_write;

            if dt_secs <= 0.0 {
                return DiskIoSnapshot::default();
            }

            DiskIoSnapshot {
                read_bytes_sec: (dr / dt_secs) as f32,
                write_bytes_sec: (dw / dt_secs) as f32,
            }
        }
    }
}

// ── Stub for non-macOS ──────────────────────────────────────────────

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::DiskIoSnapshot;

    pub struct DiskIoCollector;

    impl DiskIoCollector {
        pub fn new() -> Self {
            Self
        }

        pub fn collect(&mut self, _dt_secs: f64) -> DiskIoSnapshot {
            DiskIoSnapshot::default()
        }
    }
}

pub use platform::DiskIoCollector;
