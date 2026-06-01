extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

#[derive(Clone, Copy)]
struct FramebufferReport {
    width: usize,
    height: usize,
    stride: usize,
    bpp: usize,
    fmt: &'static str,
}

#[derive(Clone, Copy)]
struct MemoryReport {
    regions: usize,
    usable_regions: usize,
    usable_bytes: u64,
    reserved_regions: usize,
    highest_end: u64,
}

static FRAMEBUFFER: Mutex<Option<FramebufferReport>> = Mutex::new(None);
static MEMORY: Mutex<Option<MemoryReport>> = Mutex::new(None);
static SAFE_MODE: AtomicBool = AtomicBool::new(false);

pub fn record_framebuffer(
    width: usize,
    height: usize,
    stride: usize,
    bpp: usize,
    fmt: &'static str,
) {
    *FRAMEBUFFER.lock() = Some(FramebufferReport {
        width,
        height,
        stride,
        bpp,
        fmt,
    });

    if width < 1600 || height < 900 {
        SAFE_MODE.store(true, Ordering::Relaxed);
    }
}

pub fn record_memory_map(regions: &[bootloader_api::info::MemoryRegion]) {
    let mut usable_regions = 0usize;
    let mut usable_bytes = 0u64;
    let mut reserved_regions = 0usize;
    let mut highest_end = 0u64;

    for region in regions {
        highest_end = highest_end.max(region.end);
        if region.kind == bootloader_api::info::MemoryRegionKind::Usable {
            usable_regions += 1;
            usable_bytes = usable_bytes.saturating_add(region.end.saturating_sub(region.start));
        } else {
            reserved_regions += 1;
        }
    }

    *MEMORY.lock() = Some(MemoryReport {
        regions: regions.len(),
        usable_regions,
        usable_bytes,
        reserved_regions,
        highest_end,
    });
}

pub fn enable_safe_mode() {
    SAFE_MODE.store(true, Ordering::Relaxed);
}

pub fn safe_mode() -> bool {
    SAFE_MODE.load(Ordering::Relaxed)
}

pub fn lines() -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "hardware mode={}",
        if safe_mode() { "safe" } else { "normal" }
    ));

    if let Some(fb) = *FRAMEBUFFER.lock() {
        lines.push(format!(
            "framebuffer {}x{} stride={} bpp={} fmt={}",
            fb.width, fb.height, fb.stride, fb.bpp, fb.fmt
        ));
    } else {
        lines.push(String::from("framebuffer unavailable"));
    }

    if let Some(mem) = *MEMORY.lock() {
        lines.push(format!(
            "memory regions={} usable={} reserved={} usable_mib={} highest={:#x}",
            mem.regions,
            mem.usable_regions,
            mem.reserved_regions,
            mem.usable_bytes / (1024 * 1024),
            mem.highest_end
        ));
    } else {
        lines.push(String::from("memory map unavailable"));
    }

    lines.extend(storage_lines());
    lines.extend(crate::storage::root_scan_lines());
    lines.extend(crate::installer::hardware_summary_lines());
    lines.extend(crate::ahci::status_lines());
    lines.extend(crate::nvme::status_lines());
    lines.extend(crate::usb::status_lines());
    lines
}

pub fn device_lines() -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "SYS - boot/hardware coolOS {}",
        if safe_mode() { "safe-mode" } else { "normal" }
    ));
    lines.extend(storage_lines());
    lines.extend(crate::storage::root_scan_lines());
    lines.extend(crate::installer::hardware_summary_lines());
    lines.extend(crate::device_registry::lines());
    lines
}

fn storage_lines() -> Vec<String> {
    let mut lines = Vec::new();
    match crate::storage::root_disk() {
        Some(root) => lines.push(format!(
            "storage root={} layout={} base_lba={} sectors={}",
            root.device.name(),
            root.layout.name(),
            root.base_lba,
            root.sectors
        )),
        None => lines.push(String::from("storage root=missing")),
    }

    for device in crate::storage::all_devices() {
        let info = crate::storage::device_info(device);
        let state = if info.present {
            "usable"
        } else {
            "not-present"
        };
        lines.push(format!(
            "storage device={} state={} sectors={}",
            device.name(),
            state,
            info.sectors
        ));
    }
    lines
}
