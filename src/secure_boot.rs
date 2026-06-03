extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};

pub fn enabled() -> bool {
    crate::fw_cfg::secure_boot_status().is_some()
}

pub fn status() -> String {
    crate::fw_cfg::secure_boot_status()
        .unwrap_or_else(|| String::from("mode=unsigned loader=unchecked kernel=unchecked"))
}

pub fn boot_log_lines() -> Vec<String> {
    if enabled() {
        vec![format!("[secureboot] {}", status())]
    } else {
        Vec::new()
    }
}

pub fn lines() -> Vec<String> {
    vec![format!("secure_boot {}", status())]
}
