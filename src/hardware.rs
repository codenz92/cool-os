extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

const REPORT_PATH: &str = "/LOGS/HARDWARE.TXT";

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

pub fn report_path() -> &'static str {
    REPORT_PATH
}

pub fn write_report() -> Result<(), &'static str> {
    let mut report = String::from("coolOS hardware report\n");
    report.push_str(&format!("tick={}\n", crate::interrupts::ticks()));
    if let Some(dt) = crate::rtc::read_datetime() {
        report.push_str(&format!(
            "rtc={:04}-{:02}-{:02} {:02}:{:02}\n",
            dt.year, dt.month, dt.day, dt.hour, dt.minute
        ));
    }
    push_section(&mut report, "hardware", lines());
    push_section(&mut report, "devices", device_lines());

    let _ = crate::vfs::vfs_kernel_create_dir("/LOGS");
    crate::vfs::vfs_kernel_safe_write_file(REPORT_PATH, report.as_bytes())
        .map_err(|_| "hardware report write failed")?;
    crate::writeback::barrier()?;
    Ok(())
}

pub fn lines() -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "hardware mode={}",
        if safe_mode() { "safe" } else { "normal" }
    ));
    lines.push(format!("hardware report_path={}", report_path()));

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

    lines.push(primary_failure_line());
    lines.extend(readiness_lines());
    lines.extend(crate::secure_boot::lines());
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
    lines.extend(crate::secure_boot::lines());
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
            "storage device={} bus={} role={} state={} sectors={}",
            device.name(),
            device.bus_label(),
            crate::installer::device_role(device),
            state,
            info.sectors
        ));
    }
    lines
}

fn primary_failure_line() -> String {
    let reason = primary_failure_reason();
    format!(
        "hardware primary_failure={} detail={}",
        reason.code, reason.detail
    )
}

struct PrimaryFailure {
    code: &'static str,
    detail: &'static str,
}

fn primary_failure_reason() -> PrimaryFailure {
    if FRAMEBUFFER.lock().is_none() {
        return PrimaryFailure {
            code: "no-framebuffer",
            detail: "bootloader did not provide a usable framebuffer",
        };
    }

    if crate::storage::root_disk().is_none() {
        return PrimaryFailure {
            code: "no-root",
            detail: "no CoolFS root found on any probed block device",
        };
    }

    let (usb_keyboard, usb_pointer) = crate::usb::input_presence();
    if !usb_keyboard && !usb_pointer && !crate::acpi::has_8042() {
        return PrimaryFailure {
            code: "no-input",
            detail: "no USB HID input and no PS/2 fallback controller",
        };
    }

    if safe_mode() {
        return PrimaryFailure {
            code: "safe-framebuffer",
            detail: "safe-mode framebuffer fallback active",
        };
    }

    PrimaryFailure {
        code: "none",
        detail: "boot hardware checks passed",
    }
}

fn readiness_lines() -> Vec<String> {
    let mut lines = Vec::new();
    let framebuffer_ok = FRAMEBUFFER.lock().is_some();
    let root_ok = crate::storage::root_disk().is_some();
    let (usb_keyboard, usb_pointer) = crate::usb::input_presence();
    let input = if usb_keyboard && usb_pointer {
        "usb-keyboard-pointer"
    } else if usb_keyboard {
        "usb-keyboard-only"
    } else if usb_pointer {
        "usb-pointer-only"
    } else if crate::acpi::has_8042() {
        "ps2-fallback"
    } else {
        "missing"
    };
    lines.push(format!(
        "hardware readiness framebuffer={} input={} root={}",
        if framebuffer_ok { "ok" } else { "failed" },
        input,
        if root_ok { "ok" } else { "missing" },
    ));
    if !framebuffer_ok {
        lines.push(String::from(
            "boot_issue no-framebuffer failed: unavailable",
        ));
    }
    if input == "missing" {
        lines.push(String::from(
            "boot_issue no-input failed: no USB HID or PS/2 fallback detected",
        ));
    }
    if !root_ok {
        lines.push(String::from(
            "boot_issue no-root failed: no CoolFS root discovered; see storage root_scan lines",
        ));
    }
    let primary = primary_failure_reason();
    if primary.code != "none" {
        lines.push(format!(
            "boot_issue primary failed: {} ({})",
            primary.code, primary.detail
        ));
    }
    lines
}

fn push_section(report: &mut String, title: &str, lines: Vec<String>) {
    report.push_str(&format!("== {} ==\n", title));
    if lines.is_empty() {
        report.push_str("(empty)\n");
    } else {
        for line in lines {
            report.push_str(&line);
            report.push('\n');
        }
    }
}
