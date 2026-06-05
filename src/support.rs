extern crate alloc;

use alloc::{format, string::String, vec::Vec};

const BUNDLE_PATH: &str = "/LOGS/SUPPORT-BUNDLE.TXT";

pub fn bundle_path() -> &'static str {
    BUNDLE_PATH
}

pub fn write_bundle() -> Result<(), &'static str> {
    crate::device_registry::refresh_pci();
    let bundle = render_bundle();
    let _ = crate::vfs::vfs_kernel_create_dir("/LOGS");
    crate::vfs::vfs_kernel_safe_write_file(BUNDLE_PATH, bundle.as_bytes())
        .map_err(|_| "support bundle write failed")?;
    crate::writeback::barrier()?;
    Ok(())
}

fn render_bundle() -> String {
    let mut report = String::from("coolOS support bundle\n");
    report.push_str("support_bundle redacted=yes passwords=excluded private_keys=excluded\n");
    report.push_str(&format!("tick={}\n", crate::interrupts::ticks()));
    if let Some(dt) = crate::rtc::read_datetime() {
        report.push_str(&format!(
            "rtc={:04}-{:02}-{:02} {:02}:{:02}\n",
            dt.year, dt.month, dt.day, dt.hour, dt.minute
        ));
    }

    push_section(&mut report, "hardware", crate::hardware::lines());
    push_section(&mut report, "devices", crate::hardware::device_lines());
    push_section(
        &mut report,
        "installer disks",
        crate::installer::disks_lines(),
    );
    push_section(
        &mut report,
        "installer preflight",
        crate::installer::hardware_summary_lines(),
    );
    push_section(&mut report, "secure boot", crate::secure_boot::lines());
    push_section(&mut report, "sysreport", crate::sysreport::lines());
    push_section(&mut report, "boot log", crate::klog::lines());
    report
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
