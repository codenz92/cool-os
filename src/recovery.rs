extern crate alloc;

use alloc::{format, string::String, vec::Vec};

const RECOVERY_DIR: &str = "/RECOVERY";
const README_PATH: &str = "/RECOVERY/README.TXT";
const BOOT_CFG_PATH: &str = "/RECOVERY/BOOT.CFG";
const LAST_REPAIR_PATH: &str = "/RECOVERY/LAST-REPAIR.TXT";

const README: &[u8] = b"coolOS recovery\n\nCommands:\n  recovery\n  recovery repair\n  recovery rollback\n  recovery fsck-on-boot on\n  recovery fsck-on-boot off\n\nThe normal boot path is BIOS VBE framebuffer + IDE CoolFS root. Keep this directory on the root filesystem so recovery instructions survive package, update, and user changes.\n";

const BOOT_CFG: &[u8] = b"boot=normal\nroot=/\nrootfs=coolfs\nvideo=bios-vbe\nstorage=ide1\nrecovery_command=recovery repair\n";

pub fn status_lines() -> Vec<String> {
    ensure_layout();
    let settings = crate::settings_state::snapshot();
    let mut lines = alloc::vec![
        String::from("mode=normal recovery=available"),
        String::from("boot=BIOS/VBE root=/ type=coolfs storage=ide,index=1"),
        format!("manifest={}", BOOT_CFG_PATH),
        format!("fsck_on_boot={}", settings.storage_fsck_on_boot),
    ];
    if crate::vfs::vfs_kernel_read_file(README_PATH).is_some() {
        lines.push(format!("readme={}", README_PATH));
    } else {
        lines.push(String::from("readme=missing"));
    }
    if let Some(report) = crate::coolfs::check() {
        lines.push(format!(
            "coolfs ok={} root_entries={} used={}/{}",
            report.ok, report.root_entries, report.stats.used_blocks, report.stats.total_blocks
        ));
    }
    lines.extend(crate::fs_hardening::status_lines());
    lines.extend(crate::boot_health::recovery_lines());
    lines.extend(crate::services::recovery_lines());
    lines.extend(crate::updates::recovery_lines());
    lines.extend(crate::packages::recovery_lines());
    lines.extend(crate::browser_engine::recovery_lines());
    lines
}

pub fn repair_lines() -> Vec<String> {
    ensure_layout();
    let repair = crate::fs_hardening::repair();
    let boot_recovery = crate::boot_health::recovery_lines();
    let service_recovery = crate::services::recovery_lines();
    let update_recovery = crate::updates::recovery_lines();
    let package_recovery = crate::packages::recovery_lines();
    let browser_engine_recovery = crate::browser_engine::recovery_lines();
    let mut report = String::from("coolOS recovery repair report\n");
    report.push_str("boot=BIOS/VBE root=/ type=coolfs\n");
    for line in &repair {
        report.push_str(line);
        report.push('\n');
    }
    for line in &boot_recovery {
        report.push_str(line);
        report.push('\n');
    }
    for line in &service_recovery {
        report.push_str(line);
        report.push('\n');
    }
    for line in &update_recovery {
        report.push_str(line);
        report.push('\n');
    }
    for line in &package_recovery {
        report.push_str(line);
        report.push('\n');
    }
    for line in &browser_engine_recovery {
        report.push_str(line);
        report.push('\n');
    }
    let write_result = write_file(LAST_REPAIR_PATH, report.as_bytes());

    let mut lines = alloc::vec![
        String::from("recovery repair started"),
        format!("layout={}", RECOVERY_DIR),
    ];
    lines.extend(repair);
    lines.extend(boot_recovery);
    lines.extend(service_recovery);
    lines.extend(update_recovery);
    lines.extend(package_recovery);
    lines.extend(browser_engine_recovery);
    match write_result {
        Ok(()) => lines.push(format!("wrote {}", LAST_REPAIR_PATH)),
        Err(err) => lines.push(format!("write {}: {}", LAST_REPAIR_PATH, err.as_str())),
    }
    lines
}

pub fn set_fsck_on_boot(enabled: bool) -> Vec<String> {
    let ok = crate::settings_state::set("storage_fsck_on_boot", enabled);
    alloc::vec![format!(
        "storage.fsck_on_boot={} {}",
        enabled,
        if ok { "saved" } else { "failed" }
    )]
}

fn ensure_layout() {
    let _ = crate::vfs::vfs_kernel_create_dir(RECOVERY_DIR);
    ensure_file(README_PATH, README);
    ensure_file(BOOT_CFG_PATH, BOOT_CFG);
}

fn ensure_file(path: &str, data: &[u8]) {
    if crate::vfs::vfs_kernel_read_file(path).is_some() {
        return;
    }
    let _ = write_file(path, data);
}

fn write_file(path: &str, data: &[u8]) -> Result<(), crate::fat32::FsError> {
    match crate::vfs::vfs_kernel_create_file(path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
        Err(err) => return Err(err),
    }
    crate::vfs::vfs_kernel_write_file(path, data)
}
