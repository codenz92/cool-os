extern crate alloc;

use alloc::{format, string::String, vec::Vec};

const REPORT_PATH: &str = "/LOGS/SYSREPORT.TXT";

pub fn lines() -> Vec<String> {
    let mut lines = Vec::new();
    push_section(&mut lines, "kernel log", crate::klog::lines());
    push_section(&mut lines, "profiler", crate::profiler::lines());
    push_section(
        &mut lines,
        "boot health",
        crate::boot_health::status_lines(),
    );
    push_section(&mut lines, "services", crate::services::lines());
    push_section(
        &mut lines,
        "service recovery",
        crate::services::recovery_lines(),
    );
    push_section(&mut lines, "updates", crate::updates::status_lines());
    push_section(&mut lines, "tasks", crate::process_model::status_lines());
    push_section(
        &mut lines,
        "memory pressure",
        crate::memory_pressure::lines(),
    );
    push_section(
        &mut lines,
        "task memory",
        crate::scheduler::task_memory_lines(),
    );
    push_section(
        &mut lines,
        "resource limits",
        crate::resource_limits::lines(),
    );
    push_section(&mut lines, "wait queues", crate::wait_queue::lines());
    push_section(&mut lines, "vfs", crate::vfs::mount_lines());
    push_section(&mut lines, "writeback", crate::writeback::lines());
    push_section(&mut lines, "crash", crate::crashdump::detailed_lines());
    lines
}

pub fn write_report() -> Result<(), &'static str> {
    let mut report = String::from("coolOS system report\n");
    report.push_str(&format!("tick={}\n", crate::interrupts::ticks()));
    if let Some(dt) = crate::rtc::read_datetime() {
        report.push_str(&format!(
            "rtc={:04}-{:02}-{:02} {:02}:{:02}\n",
            dt.year, dt.month, dt.day, dt.hour, dt.minute
        ));
    }
    for line in lines() {
        report.push_str(&line);
        report.push('\n');
    }
    let _ = crate::vfs::vfs_kernel_create_dir("/LOGS");
    crate::vfs::vfs_kernel_safe_write_file(REPORT_PATH, report.as_bytes())
        .map_err(|_| "sysreport write failed")?;
    crate::writeback::barrier()?;
    Ok(())
}

pub fn report_path() -> &'static str {
    REPORT_PATH
}

fn push_section(out: &mut Vec<String>, title: &str, lines: Vec<String>) {
    out.push(format!("== {} ==", title));
    if lines.is_empty() {
        out.push(String::from("(empty)"));
    } else {
        out.extend(lines);
    }
}
