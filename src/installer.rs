extern crate alloc;

use alloc::{format, string::String, vec::Vec};

use crate::ata::{AtaDeviceInfo, IdeDevice};

const SOURCE_DEVICE: IdeDevice = IdeDevice::Ide0Slave;
const BOOT_DEVICE: IdeDevice = IdeDevice::Ide0Master;
const COOLFS_PARTITION_INDEX: usize = 2;

#[derive(Clone, Copy)]
struct InstallLayout {
    boot_copy_sectors: u32,
    root_start_lba: u32,
    root_sectors: u32,
    total_required_sectors: u32,
}

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
    let boot_info = crate::ata::device_info(BOOT_DEVICE);
    if !boot_info.present {
        lines.push(String::from("install: boot disk not present"));
        return lines;
    }
    let target_info = crate::ata::device_info(target);
    let Some(layout) = install_layout(source_info.sectors, target_info.sectors, &mut lines) else {
        return lines;
    };

    lines.push(format!(
        "source_boot={} sectors={}",
        BOOT_DEVICE.name(),
        boot_info.sectors
    ));
    lines.push(format!(
        "source_root={} sectors={}",
        SOURCE_DEVICE.name(),
        source_info.sectors
    ));
    lines.push(format!(
        "target={} sectors={}",
        target.name(),
        target_info.sectors
    ));
    lines.push(format!(
        "layout=self-boot mbr_type=0x{:02x} boot_copy_sectors={} root_start_lba={} root_sectors={} required_sectors={}",
        crate::disk_layout::COOLFS_PARTITION_TYPE,
        layout.boot_copy_sectors,
        layout.root_start_lba,
        layout.root_sectors,
        layout.total_required_sectors
    ));

    let mut sector = [0u8; 512];
    lines.push(format!(
        "copy boot started sectors={}",
        layout.boot_copy_sectors
    ));
    for lba in 0..layout.boot_copy_sectors {
        if !crate::ata::read_sector_from(BOOT_DEVICE, lba, &mut sector) {
            lines.push(format!("install: boot read failed lba={}", lba));
            return lines;
        }
        if !crate::ata::write_sector_to(target, lba, &sector) {
            lines.push(format!("install: target boot write failed lba={}", lba));
            return lines;
        }
    }
    lines.push(format!(
        "copy boot complete sectors={}",
        layout.boot_copy_sectors
    ));

    if !write_target_mbr(target, layout, &mut lines) {
        return lines;
    }

    lines.push(format!("copy root started sectors={}", layout.root_sectors));
    for lba in 0..layout.root_sectors {
        if !crate::ata::read_sector_from(SOURCE_DEVICE, lba, &mut sector) {
            lines.push(format!("install: source read failed lba={}", lba));
            return lines;
        }
        let Some(target_lba) = layout.root_start_lba.checked_add(lba) else {
            lines.push(format!("install: target LBA overflow lba={}", lba));
            return lines;
        };
        if !crate::ata::write_sector_to(target, target_lba, &sector) {
            lines.push(format!(
                "install: target root write failed lba={}",
                target_lba
            ));
            return lines;
        }
    }

    lines.push(format!(
        "copy root complete sectors={}",
        layout.root_sectors
    ));
    if crate::ata::flush_device(target) {
        lines.push(String::from("flush=ok"));
    } else {
        lines.push(String::from("install: target flush failed"));
        return lines;
    }

    let verified = verify_self_boot_device(target, layout, &mut lines);
    if verified {
        lines.push(format!("install complete target={}", target.name()));
        lines.push(String::from("reboot_with_target_as_boot_disk=ide0-master"));
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
    let Some(layout) = installed_layout_from_target(target, source_info.sectors, &mut lines) else {
        return lines;
    };
    if target_info.sectors < layout.total_required_sectors {
        lines.push(format!(
            "verify: target too small required_sectors={} target_sectors={}",
            layout.total_required_sectors, target_info.sectors
        ));
        return lines;
    }
    verify_self_boot_device(target, layout, &mut lines);
    lines
}

fn verify_self_boot_device(
    target: IdeDevice,
    layout: InstallLayout,
    lines: &mut Vec<String>,
) -> bool {
    lines.push(format!(
        "verify started layout=self-boot boot_sectors={} root_sectors={}",
        layout.boot_copy_sectors, layout.root_sectors
    ));
    if !verify_target_mbr(target, layout, lines) {
        return false;
    }
    let mut source = [0u8; 512];
    let mut dest = [0u8; 512];
    for lba in 1..layout.boot_copy_sectors {
        if !crate::ata::read_sector_from(BOOT_DEVICE, lba, &mut source) {
            lines.push(format!("verify: boot read failed lba={}", lba));
            return false;
        }
        if !crate::ata::read_sector_from(target, lba, &mut dest) {
            lines.push(format!("verify: target boot read failed lba={}", lba));
            return false;
        }
        if source != dest {
            lines.push(format!("verify: boot mismatch lba={}", lba));
            return false;
        }
    }
    let mut source = [0u8; 512];
    let mut dest = [0u8; 512];
    for lba in 0..layout.root_sectors {
        if !crate::ata::read_sector_from(SOURCE_DEVICE, lba, &mut source) {
            lines.push(format!("verify: source read failed lba={}", lba));
            return false;
        }
        let Some(target_lba) = layout.root_start_lba.checked_add(lba) else {
            lines.push(format!("verify: target LBA overflow lba={}", lba));
            return false;
        };
        if !crate::ata::read_sector_from(target, target_lba, &mut dest) {
            lines.push(format!(
                "verify: target root read failed lba={}",
                target_lba
            ));
            return false;
        }
        if source != dest {
            lines.push(format!("verify: root mismatch lba={}", lba));
            return false;
        }
    }
    lines.push(format!(
        "verify=ok layout=self-boot boot_sectors={} root_sectors={}",
        layout.boot_copy_sectors, layout.root_sectors
    ));
    true
}

fn install_layout(
    root_sectors: u32,
    target_sectors: u32,
    lines: &mut Vec<String>,
) -> Option<InstallLayout> {
    let mut boot_mbr = [0u8; 512];
    if !crate::ata::read_sector_from(BOOT_DEVICE, 0, &mut boot_mbr) {
        lines.push(String::from("install: boot MBR read failed"));
        return None;
    }
    let Some(partitions) = crate::disk_layout::parse_mbr(&boot_mbr) else {
        lines.push(String::from("install: boot disk has no valid MBR"));
        return None;
    };
    let Some(boot_end) = crate::disk_layout::boot_area_end_lba(&partitions) else {
        lines.push(String::from("install: boot partition layout overflow"));
        return None;
    };
    let Some(root_start_lba) =
        crate::disk_layout::align_up_lba(boot_end, crate::disk_layout::INSTALL_ALIGNMENT_SECTORS)
    else {
        lines.push(String::from("install: root partition alignment overflow"));
        return None;
    };
    let Some(total_required_sectors) = root_start_lba.checked_add(root_sectors) else {
        lines.push(String::from("install: target layout overflow"));
        return None;
    };
    if target_sectors < total_required_sectors {
        lines.push(format!(
            "install: target too small required_sectors={} target_sectors={} source_root_sectors={} boot_copy_sectors={} root_start_lba={}",
            total_required_sectors, target_sectors, root_sectors, boot_end, root_start_lba
        ));
        return None;
    }
    Some(InstallLayout {
        boot_copy_sectors: boot_end,
        root_start_lba,
        root_sectors,
        total_required_sectors,
    })
}

fn installed_layout_from_target(
    target: IdeDevice,
    expected_root_sectors: u32,
    lines: &mut Vec<String>,
) -> Option<InstallLayout> {
    let mut target_mbr = [0u8; 512];
    if !crate::ata::read_sector_from(target, 0, &mut target_mbr) {
        lines.push(String::from("verify: target MBR read failed"));
        return None;
    }
    let Some(partitions) = crate::disk_layout::parse_mbr(&target_mbr) else {
        lines.push(String::from("verify: target has no valid MBR"));
        return None;
    };
    let Some(boot_end) = crate::disk_layout::boot_area_end_lba(&partitions) else {
        lines.push(String::from("verify: target boot layout overflow"));
        return None;
    };
    let Some(root_partition) =
        crate::disk_layout::find_partition(&partitions, crate::disk_layout::COOLFS_PARTITION_TYPE)
    else {
        lines.push(format!(
            "verify: target missing CoolFS partition type=0x{:02x}",
            crate::disk_layout::COOLFS_PARTITION_TYPE
        ));
        return None;
    };
    if root_partition.sectors != expected_root_sectors {
        lines.push(format!(
            "verify: root size mismatch expected={} actual={}",
            expected_root_sectors, root_partition.sectors
        ));
        return None;
    }
    let Some(total_required_sectors) = root_partition.end_lba() else {
        lines.push(String::from("verify: root partition overflow"));
        return None;
    };
    Some(InstallLayout {
        boot_copy_sectors: boot_end,
        root_start_lba: root_partition.starting_lba,
        root_sectors: root_partition.sectors,
        total_required_sectors,
    })
}

fn write_target_mbr(target: IdeDevice, layout: InstallLayout, lines: &mut Vec<String>) -> bool {
    let mut mbr = [0u8; 512];
    if !crate::ata::read_sector_from(BOOT_DEVICE, 0, &mut mbr) {
        lines.push(String::from("install: boot MBR read failed"));
        return false;
    }
    if crate::disk_layout::write_partition_entry(
        &mut mbr,
        COOLFS_PARTITION_INDEX,
        0,
        crate::disk_layout::COOLFS_PARTITION_TYPE,
        layout.root_start_lba,
        layout.root_sectors,
    )
    .is_none()
    {
        lines.push(String::from("install: MBR partition patch failed"));
        return false;
    }
    if !crate::ata::write_sector_to(target, 0, &mbr) {
        lines.push(String::from("install: target MBR write failed"));
        return false;
    }
    lines.push(format!(
        "mbr patched partition={} type=0x{:02x} start={} sectors={}",
        COOLFS_PARTITION_INDEX + 1,
        crate::disk_layout::COOLFS_PARTITION_TYPE,
        layout.root_start_lba,
        layout.root_sectors
    ));
    true
}

fn verify_target_mbr(target: IdeDevice, layout: InstallLayout, lines: &mut Vec<String>) -> bool {
    let mut source_mbr = [0u8; 512];
    let mut target_mbr = [0u8; 512];
    if !crate::ata::read_sector_from(BOOT_DEVICE, 0, &mut source_mbr) {
        lines.push(String::from("verify: boot MBR read failed"));
        return false;
    }
    if !crate::ata::read_sector_from(target, 0, &mut target_mbr) {
        lines.push(String::from("verify: target MBR read failed"));
        return false;
    }
    if source_mbr[..446] != target_mbr[..446] {
        lines.push(String::from("verify: MBR boot code mismatch"));
        return false;
    }
    let Some(partitions) = crate::disk_layout::parse_mbr(&target_mbr) else {
        lines.push(String::from("verify: target has no valid MBR"));
        return false;
    };
    let Some(root_partition) =
        crate::disk_layout::find_partition(&partitions, crate::disk_layout::COOLFS_PARTITION_TYPE)
    else {
        lines.push(String::from("verify: CoolFS partition missing"));
        return false;
    };
    if root_partition.starting_lba != layout.root_start_lba
        || root_partition.sectors != layout.root_sectors
    {
        lines.push(format!(
            "verify: CoolFS partition mismatch start={} sectors={}",
            root_partition.starting_lba, root_partition.sectors
        ));
        return false;
    }
    lines.push(String::from("verify boot=ok"));
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
