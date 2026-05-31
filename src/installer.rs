extern crate alloc;

use alloc::{format, string::String, vec::Vec};

use crate::ata::{AtaDeviceInfo, IdeDevice};

const SOURCE_DEVICE: IdeDevice = IdeDevice::Ide0Slave;
const BOOT_DEVICE: IdeDevice = IdeDevice::Ide0Master;

pub fn source_device() -> IdeDevice {
    SOURCE_DEVICE
}

pub fn disks_lines() -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "installer mode={}",
        if crate::fw_cfg::installer_mode() {
            "active"
        } else {
            "inactive"
        }
    ));
    for device in crate::ata::all_devices() {
        let info = crate::ata::device_info(device);
        lines.push(format_device_line(info));
    }
    lines
}

pub fn install_to_device_name(name: &str) -> Vec<String> {
    let Some(target) = IdeDevice::parse(name) else {
        return alloc::vec![format!("install: unknown disk {}", name)];
    };
    install_to_device(target)
}

pub fn verify_device_name(name: &str) -> Vec<String> {
    let Some(target) = IdeDevice::parse(name) else {
        return alloc::vec![format!("install: unknown disk {}", name)];
    };
    verify_device(target)
}

pub fn install_to_device(target: IdeDevice) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("install target={}", target.name()));
    if let Err(err) = validate_install_target(target) {
        lines.push(format!("install: {}", err));
        lines.extend(disks_lines());
        return lines;
    }

    let source_info = crate::ata::device_info(SOURCE_DEVICE);
    if !source_info.present {
        lines.push(String::from("install: source disk not present"));
        return lines;
    }
    let target_info = crate::ata::device_info(target);
    let sectors = source_info.sectors;
    if target_info.sectors < sectors {
        lines.push(format!(
            "install: target too small source_sectors={} target_sectors={}",
            sectors, target_info.sectors
        ));
        return lines;
    }

    lines.push(format!(
        "source={} sectors={}",
        SOURCE_DEVICE.name(),
        sectors
    ));
    lines.push(format!(
        "target={} sectors={}",
        target.name(),
        target_info.sectors
    ));
    lines.push(format!("copy started sectors={}", sectors));

    let mut sector = [0u8; 512];
    for lba in 0..sectors {
        if !crate::ata::read_sector_from(SOURCE_DEVICE, lba, &mut sector) {
            lines.push(format!("install: source read failed lba={}", lba));
            return lines;
        }
        if !crate::ata::write_sector_to(target, lba, &sector) {
            lines.push(format!("install: target write failed lba={}", lba));
            return lines;
        }
    }

    lines.push(format!("copy complete sectors={}", sectors));
    if crate::ata::flush_device(target) {
        lines.push(String::from("flush=ok"));
    } else {
        lines.push(String::from("install: target flush failed"));
        return lines;
    }

    let verified = verify_device_sectors(target, sectors, &mut lines);
    if verified {
        lines.push(format!("install complete target={}", target.name()));
        lines.push(String::from("reboot_with_target_as_root=ide0-slave"));
    }
    lines
}

pub fn verify_device(target: IdeDevice) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("verify target={}", target.name()));
    if let Err(err) = validate_install_target(target) {
        lines.push(format!("verify: {}", err));
        return lines;
    }
    let source_info = crate::ata::device_info(SOURCE_DEVICE);
    if !source_info.present {
        lines.push(String::from("verify: source disk not present"));
        return lines;
    }
    let target_info = crate::ata::device_info(target);
    let sectors = source_info.sectors;
    if target_info.sectors < sectors {
        lines.push(format!(
            "verify: target too small source_sectors={} target_sectors={}",
            sectors, target_info.sectors
        ));
        return lines;
    }
    verify_device_sectors(target, sectors, &mut lines);
    lines
}

fn verify_device_sectors(target: IdeDevice, sectors: u32, lines: &mut Vec<String>) -> bool {
    lines.push(format!("verify started sectors={}", sectors));
    let mut source = [0u8; 512];
    let mut dest = [0u8; 512];
    for lba in 0..sectors {
        if !crate::ata::read_sector_from(SOURCE_DEVICE, lba, &mut source) {
            lines.push(format!("verify: source read failed lba={}", lba));
            return false;
        }
        if !crate::ata::read_sector_from(target, lba, &mut dest) {
            lines.push(format!("verify: target read failed lba={}", lba));
            return false;
        }
        if source != dest {
            lines.push(format!("verify: mismatch lba={}", lba));
            return false;
        }
    }
    lines.push(format!("verify=ok sectors={}", sectors));
    true
}

fn validate_install_target(target: IdeDevice) -> Result<(), &'static str> {
    if target == SOURCE_DEVICE {
        return Err("refusing to overwrite mounted root disk");
    }
    if target == BOOT_DEVICE {
        return Err("refusing to overwrite boot disk");
    }
    match target {
        IdeDevice::Ide1Master | IdeDevice::Ide1Slave => {}
        IdeDevice::Ide0Master | IdeDevice::Ide0Slave => {
            return Err("target must be on secondary IDE bus");
        }
    }
    let info = crate::ata::device_info(target);
    if !info.present {
        return Err("target disk not present");
    }
    Ok(())
}

fn format_device_line(info: AtaDeviceInfo) -> String {
    let role = if info.device == BOOT_DEVICE {
        "boot"
    } else if info.device == SOURCE_DEVICE {
        "root"
    } else {
        "target"
    };
    let protected = info.device == BOOT_DEVICE || info.device == SOURCE_DEVICE;
    let installable = info.present
        && !protected
        && matches!(info.device, IdeDevice::Ide1Master | IdeDevice::Ide1Slave);
    format!(
        "{} present={} sectors={} role={} protected={} installable={}",
        info.device.name(),
        if info.present { "yes" } else { "no" },
        info.sectors,
        role,
        if protected { "yes" } else { "no" },
        if installable { "yes" } else { "no" }
    )
}
