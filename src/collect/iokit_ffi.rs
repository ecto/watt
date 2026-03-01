//! Shared IOKit FFI declarations used by apple_gpu and disk collectors.

#![allow(non_snake_case, dead_code)]

use std::os::raw::c_void;

use core_foundation::base::CFAllocatorRef;
use core_foundation::dictionary::CFMutableDictionaryRef;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    pub fn IOServiceMatching(name: *const i8) -> CFMutableDictionaryRef;
    pub fn IOServiceGetMatchingServices(
        mainPort: u32,
        matching: CFMutableDictionaryRef,
        existing: *mut u32,
    ) -> i32;
    pub fn IOIteratorNext(iterator: u32) -> u32;
    pub fn IORegistryEntryGetName(entry: u32, name: *mut i8) -> i32;
    pub fn IORegistryEntryCreateCFProperties(
        entry: u32,
        properties: *mut CFMutableDictionaryRef,
        allocator: CFAllocatorRef,
        options: u32,
    ) -> i32;
    pub fn IOObjectRelease(obj: u32) -> u32;
    pub fn IOServiceOpen(device: u32, a: u32, b: u32, c: *mut u32) -> i32;
    pub fn IOServiceClose(conn: u32) -> i32;
    pub fn IOConnectCallStructMethod(
        conn: u32,
        selector: u32,
        ival: *const c_void,
        isize: usize,
        oval: *mut c_void,
        osize: *mut usize,
    ) -> i32;
}

extern "C" {
    pub fn mach_task_self() -> u32;
}
