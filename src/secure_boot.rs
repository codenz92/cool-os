extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};
use core::slice;
use spin::Mutex;

const BOOT_STATUS_MAGIC: &[u8] = b"COOLOS_BOOT_STATUS v1\n";
const MAX_BOOT_STATUS_LEN: u64 = 4096;

static LOADER_STATUS: Mutex<Option<String>> = Mutex::new(None);

pub fn init_from_boot_info(ramdisk_addr: Option<u64>, ramdisk_len: u64) {
    if ramdisk_len == 0 || ramdisk_len > MAX_BOOT_STATUS_LEN {
        return;
    }
    let Some(addr) = ramdisk_addr else {
        return;
    };
    if addr == 0 {
        return;
    }

    let bytes = unsafe { slice::from_raw_parts(addr as *const u8, ramdisk_len as usize) };
    if !bytes.starts_with(BOOT_STATUS_MAGIC) {
        return;
    }
    let body = &bytes[BOOT_STATUS_MAGIC.len()..];
    let end = body
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(body.len());
    let body = &body[..end];
    let Ok(text) = core::str::from_utf8(body) else {
        return;
    };
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    *LOADER_STATUS.lock() = Some(String::from(text));
}

pub fn enabled() -> bool {
    crate::fw_cfg::secure_boot_status().is_some() || LOADER_STATUS.lock().is_some()
}

pub fn status() -> String {
    crate::fw_cfg::secure_boot_status()
        .or_else(|| LOADER_STATUS.lock().clone())
        .unwrap_or_else(|| {
            String::from(
                "mode=unknown/unsigned loader=unchecked kernel=unchecked enforcement=unknown",
            )
        })
}

pub fn source() -> &'static str {
    if crate::fw_cfg::secure_boot_status().is_some() {
        "qemu-fw_cfg"
    } else if LOADER_STATUS.lock().is_some() {
        "uefi-loader"
    } else {
        "fallback"
    }
}

pub fn boot_log_lines() -> Vec<String> {
    if enabled() {
        vec![format!("[secureboot] {}", status())]
    } else {
        Vec::new()
    }
}

pub fn lines() -> Vec<String> {
    vec![
        format!("secure_boot {}", status()),
        format!("secure_boot_source {}", source()),
    ]
}
