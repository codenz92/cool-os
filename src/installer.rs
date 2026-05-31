extern crate alloc;

use alloc::{format, string::String, vec::Vec};

use crate::ata::IdeDevice;

const SOURCE_DEVICE: IdeDevice = IdeDevice::Ide0Slave;
const BOOT_DEVICE: IdeDevice = IdeDevice::Ide0Master;
const COOLFS_PARTITION_INDEX: usize = 2;
const GUI_INSTALL_SECTOR_BUDGET: u32 = 8192;
const ROOT_METADATA_REFRESH_SECTORS: u32 = 128;

#[derive(Clone, Copy)]
pub struct InstallLayout {
    pub boot_copy_sectors: u32,
    pub root_start_lba: u32,
    pub root_sectors: u32,
    pub total_required_sectors: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TargetState {
    Missing,
    Blank,
    DirectCoolFs,
    Mbr,
    SelfBoot,
    Unknown,
}

impl TargetState {
    pub const fn label(self) -> &'static str {
        match self {
            TargetState::Missing => "missing",
            TargetState::Blank => "blank",
            TargetState::DirectCoolFs => "direct-coolfs",
            TargetState::Mbr => "mbr",
            TargetState::SelfBoot => "self-boot",
            TargetState::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy)]
pub struct InstallPlan {
    pub target: IdeDevice,
    pub target_present: bool,
    pub target_sectors: u32,
    pub source_boot: IdeDevice,
    pub source_boot_present: bool,
    pub source_boot_sectors: u32,
    pub source_root: IdeDevice,
    pub source_root_present: bool,
    pub source_root_sectors: u32,
    pub layout: Option<InstallLayout>,
    pub state: TargetState,
    pub protected: bool,
    pub installable: bool,
    pub reason: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InstallerJobPhase {
    CopyBoot,
    PatchMbr,
    CopyRoot,
    Flush,
    VerifyMbr,
    VerifyBoot,
    VerifyRoot,
    Complete,
    Failed,
}

impl InstallerJobPhase {
    pub const fn label(self) -> &'static str {
        match self {
            InstallerJobPhase::CopyBoot => "Copying boot files",
            InstallerJobPhase::PatchMbr => "Writing installer layout",
            InstallerJobPhase::CopyRoot => "Copying coolOS root",
            InstallerJobPhase::Flush => "Flushing target disk",
            InstallerJobPhase::VerifyMbr => "Verifying boot layout",
            InstallerJobPhase::VerifyBoot => "Verifying boot files",
            InstallerJobPhase::VerifyRoot => "Verifying coolOS root",
            InstallerJobPhase::Complete => "Installation complete",
            InstallerJobPhase::Failed => "Installation failed",
        }
    }
}

pub struct InstallerJob {
    target: IdeDevice,
    layout: InstallLayout,
    phase: InstallerJobPhase,
    cursor: u32,
    completed_units: u32,
    total_units: u32,
    boot_checksums: Vec<u32>,
    root_checksums: Vec<u32>,
    message: String,
}

pub fn source_device() -> IdeDevice {
    SOURCE_DEVICE
}

pub fn boot_device() -> IdeDevice {
    BOOT_DEVICE
}

pub fn default_target_device() -> IdeDevice {
    IdeDevice::Ide1Master
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
        let plan = install_plan(device);
        lines.push(format_device_line(&plan));
    }
    lines
}

pub fn plan_device_name(name: &str) -> Vec<String> {
    let Some(target) = IdeDevice::parse(name) else {
        return alloc::vec![format!("install plan: unknown disk {}", name)];
    };
    plan_device(target)
}

pub fn plan_device(target: IdeDevice) -> Vec<String> {
    let plan = install_plan(target);
    plan_lines(&plan)
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

pub fn install_plan(target: IdeDevice) -> InstallPlan {
    let boot_info = crate::ata::device_info(BOOT_DEVICE);
    let source_info = crate::ata::device_info(SOURCE_DEVICE);
    let target_info = crate::ata::device_info(target);
    let protected = target == BOOT_DEVICE || target == SOURCE_DEVICE;
    let state = target_state(target, target_info.present);

    let mut layout = None;
    let mut reason = "ready";
    let mut installable = true;

    if target == BOOT_DEVICE {
        reason = "refusing to overwrite boot disk";
        installable = false;
    } else if target == SOURCE_DEVICE {
        reason = "refusing to overwrite mounted root disk";
        installable = false;
    } else if !matches!(target, IdeDevice::Ide1Master | IdeDevice::Ide1Slave) {
        reason = "target must be on secondary IDE bus";
        installable = false;
    } else if !target_info.present {
        reason = "target disk not present";
        installable = false;
    } else if !source_info.present {
        reason = "source disk not present";
        installable = false;
    } else if !boot_info.present {
        reason = "boot disk not present";
        installable = false;
    } else {
        match compute_install_layout(source_info.sectors, target_info.sectors) {
            Ok(computed) => {
                layout = Some(computed);
            }
            Err(err) => {
                reason = err;
                installable = false;
            }
        }
    }

    InstallPlan {
        target,
        target_present: target_info.present,
        target_sectors: target_info.sectors,
        source_boot: boot_info.device,
        source_boot_present: boot_info.present,
        source_boot_sectors: boot_info.sectors,
        source_root: source_info.device,
        source_root_present: source_info.present,
        source_root_sectors: source_info.sectors,
        layout,
        state,
        protected,
        installable,
        reason,
    }
}

pub fn install_to_device(target: IdeDevice) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("install target={}", target.name()));
    let plan = install_plan(target);
    if let Err(err) = validate_install_target(&plan) {
        lines.push(format!("install: {}", err));
        lines.extend(disks_lines());
        return lines;
    }

    let Some(layout) = plan.layout else {
        lines.push(format!("install: {}", plan.reason));
        return lines;
    };
    if let Err(err) = stabilize_source_for_install() {
        lines.push(format!("install: source flush failed: {}", err));
        return lines;
    }
    lines.push(String::from("source_flush=ok"));

    lines.push(format!(
        "source_boot={} sectors={}",
        plan.source_boot.name(),
        plan.source_boot_sectors
    ));
    lines.push(format!(
        "source_root={} sectors={}",
        plan.source_root.name(),
        plan.source_root_sectors
    ));
    lines.push(format!(
        "target={} sectors={}",
        target.name(),
        plan.target_sectors
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
    if !refresh_target_root_prefix(target, layout, &mut lines) {
        return lines;
    }
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
    let plan = install_plan(target);
    if let Err(err) = validate_install_target(&plan) {
        lines.push(format!("verify: {}", err));
        return lines;
    }
    let Some(layout) = installed_layout_from_target(target, plan.source_root_sectors, &mut lines)
    else {
        return lines;
    };
    if plan.target_sectors < layout.total_required_sectors {
        lines.push(format!(
            "verify: target too small required_sectors={} target_sectors={}",
            layout.total_required_sectors, plan.target_sectors
        ));
        return lines;
    }
    verify_self_boot_device(target, layout, &mut lines);
    lines
}

pub fn start_gui_install(target: IdeDevice) -> Result<InstallerJob, Vec<String>> {
    let plan = install_plan(target);
    if !plan.installable {
        return Err(plan_lines(&plan));
    }
    let Some(layout) = plan.layout else {
        return Err(plan_lines(&plan));
    };
    if let Err(err) = stabilize_source_for_install() {
        return Err(alloc::vec![format!(
            "install: source flush failed: {}",
            err
        )]);
    }
    Ok(InstallerJob::new(target, layout))
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

impl InstallerJob {
    fn new(target: IdeDevice, layout: InstallLayout) -> Self {
        let total_units = layout
            .boot_copy_sectors
            .saturating_add(layout.root_sectors)
            .saturating_add(1)
            .saturating_add(1)
            .saturating_add(layout.boot_copy_sectors.saturating_sub(1))
            .saturating_add(layout.root_sectors)
            .max(1);
        Self {
            target,
            layout,
            phase: InstallerJobPhase::CopyBoot,
            cursor: 0,
            completed_units: 0,
            total_units,
            boot_checksums: Vec::with_capacity(layout.boot_copy_sectors as usize),
            root_checksums: Vec::with_capacity(layout.root_sectors as usize),
            message: String::from("Starting installer"),
        }
    }

    pub fn layout(&self) -> InstallLayout {
        self.layout
    }

    pub fn phase_label(&self) -> &'static str {
        self.phase.label()
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn completed_units(&self) -> u32 {
        self.completed_units
    }

    pub fn total_units(&self) -> u32 {
        self.total_units
    }

    pub fn progress_percent(&self) -> u32 {
        self.completed_units.saturating_mul(100) / self.total_units.max(1)
    }

    pub fn is_running(&self) -> bool {
        !matches!(
            self.phase,
            InstallerJobPhase::Complete | InstallerJobPhase::Failed
        )
    }

    pub fn is_complete(&self) -> bool {
        self.phase == InstallerJobPhase::Complete
    }

    pub fn is_failed(&self) -> bool {
        self.phase == InstallerJobPhase::Failed
    }

    pub fn tick_default_budget(&mut self) -> bool {
        self.tick(GUI_INSTALL_SECTOR_BUDGET)
    }

    pub fn tick(&mut self, budget: u32) -> bool {
        if !self.is_running() {
            return false;
        }
        let mut remaining = budget.max(1);
        let mut changed = false;
        let mut source = [0u8; 512];
        let mut dest = [0u8; 512];
        while remaining > 0 && self.is_running() {
            match self.phase {
                InstallerJobPhase::CopyBoot => {
                    if self.cursor >= self.layout.boot_copy_sectors {
                        self.phase = InstallerJobPhase::PatchMbr;
                        self.cursor = 0;
                        self.message = String::from("Boot files copied");
                        changed = true;
                        continue;
                    }
                    let lba = self.cursor;
                    if !crate::ata::read_sector_from(BOOT_DEVICE, lba, &mut source) {
                        self.fail(format!("boot read failed lba={}", lba));
                        return true;
                    }
                    self.boot_checksums.push(sector_checksum(&source));
                    if !crate::ata::write_sector_to(self.target, lba, &source) {
                        self.fail(format!("target boot write failed lba={}", lba));
                        return true;
                    }
                    self.cursor = self.cursor.saturating_add(1);
                    self.completed_units = self.completed_units.saturating_add(1);
                    self.message = format!(
                        "Copied boot sector {}/{}",
                        self.cursor, self.layout.boot_copy_sectors
                    );
                    remaining -= 1;
                    changed = true;
                }
                InstallerJobPhase::PatchMbr => {
                    let mut lines = Vec::new();
                    if !write_target_mbr(self.target, self.layout, &mut lines) {
                        let message = lines
                            .last()
                            .cloned()
                            .unwrap_or_else(|| String::from("target MBR write failed"));
                        self.fail(message);
                        return true;
                    }
                    self.phase = InstallerJobPhase::CopyRoot;
                    self.cursor = 0;
                    self.message = String::from("Installer layout written");
                    changed = true;
                }
                InstallerJobPhase::CopyRoot => {
                    if self.cursor >= self.layout.root_sectors {
                        if let Err(message) = self.refresh_root_prefix() {
                            self.fail(message);
                            return true;
                        }
                        self.phase = InstallerJobPhase::Flush;
                        self.cursor = 0;
                        self.message = String::from("coolOS root copied and stabilized");
                        changed = true;
                        continue;
                    }
                    let lba = self.cursor;
                    if !crate::ata::read_sector_from(SOURCE_DEVICE, lba, &mut source) {
                        self.fail(format!("source read failed lba={}", lba));
                        return true;
                    }
                    self.root_checksums.push(sector_checksum(&source));
                    let Some(target_lba) = self.layout.root_start_lba.checked_add(lba) else {
                        self.fail(format!("target LBA overflow lba={}", lba));
                        return true;
                    };
                    if !crate::ata::write_sector_to(self.target, target_lba, &source) {
                        self.fail(format!("target root write failed lba={}", target_lba));
                        return true;
                    }
                    self.cursor = self.cursor.saturating_add(1);
                    self.completed_units = self.completed_units.saturating_add(1);
                    self.message = format!(
                        "Copied root sector {}/{}",
                        self.cursor, self.layout.root_sectors
                    );
                    remaining -= 1;
                    changed = true;
                }
                InstallerJobPhase::Flush => {
                    if crate::ata::flush_device(self.target) {
                        self.completed_units = self.completed_units.saturating_add(1);
                        self.phase = InstallerJobPhase::VerifyMbr;
                        self.cursor = 0;
                        self.message = String::from("Target disk flushed");
                        changed = true;
                    } else {
                        self.fail(String::from("target flush failed"));
                        return true;
                    }
                }
                InstallerJobPhase::VerifyMbr => {
                    let mut lines = Vec::new();
                    if !verify_target_mbr(self.target, self.layout, &mut lines) {
                        let message = lines
                            .last()
                            .cloned()
                            .unwrap_or_else(|| String::from("target MBR verify failed"));
                        self.fail(message);
                        return true;
                    }
                    self.completed_units = self.completed_units.saturating_add(1);
                    self.phase = InstallerJobPhase::VerifyBoot;
                    self.cursor = 1;
                    self.message = String::from("Boot layout verified");
                    changed = true;
                }
                InstallerJobPhase::VerifyBoot => {
                    if self.cursor >= self.layout.boot_copy_sectors {
                        self.phase = InstallerJobPhase::VerifyRoot;
                        self.cursor = 0;
                        self.message = String::from("Boot files verified");
                        changed = true;
                        continue;
                    }
                    let lba = self.cursor;
                    if !crate::ata::read_sector_from(self.target, lba, &mut dest) {
                        self.fail(format!("target boot verify read failed lba={}", lba));
                        return true;
                    }
                    let expected = self
                        .boot_checksums
                        .get(lba as usize)
                        .copied()
                        .unwrap_or_default();
                    if sector_checksum(&dest) != expected {
                        self.fail(format!("boot mismatch lba={}", lba));
                        return true;
                    }
                    self.cursor = self.cursor.saturating_add(1);
                    self.completed_units = self.completed_units.saturating_add(1);
                    self.message = format!(
                        "Verified boot sector {}/{}",
                        self.cursor, self.layout.boot_copy_sectors
                    );
                    remaining -= 1;
                    changed = true;
                }
                InstallerJobPhase::VerifyRoot => {
                    if self.cursor >= self.layout.root_sectors {
                        self.phase = InstallerJobPhase::Complete;
                        self.completed_units = self.total_units;
                        self.message = format!("Installed {}", self.target.name());
                        crate::println!(
                            "[install] gui install complete target={}",
                            self.target.name()
                        );
                        return true;
                    }
                    let lba = self.cursor;
                    let Some(target_lba) = self.layout.root_start_lba.checked_add(lba) else {
                        self.fail(format!("target verify LBA overflow lba={}", lba));
                        return true;
                    };
                    if !crate::ata::read_sector_from(self.target, target_lba, &mut dest) {
                        self.fail(format!("target root verify read failed lba={}", target_lba));
                        return true;
                    }
                    let expected = self
                        .root_checksums
                        .get(lba as usize)
                        .copied()
                        .unwrap_or_default();
                    if sector_checksum(&dest) != expected {
                        self.fail(format!("root mismatch lba={}", lba));
                        return true;
                    }
                    self.cursor = self.cursor.saturating_add(1);
                    self.completed_units = self.completed_units.saturating_add(1);
                    self.message = format!(
                        "Verified root sector {}/{}",
                        self.cursor, self.layout.root_sectors
                    );
                    remaining -= 1;
                    changed = true;
                }
                InstallerJobPhase::Complete | InstallerJobPhase::Failed => break,
            }
        }
        changed
    }

    fn refresh_root_prefix(&mut self) -> Result<(), String> {
        stabilize_source_for_install().map_err(|err| format!("source refresh failed: {}", err))?;
        let limit = self.layout.root_sectors.min(ROOT_METADATA_REFRESH_SECTORS);
        let mut sector = [0u8; 512];
        for lba in 0..limit {
            if !crate::ata::read_sector_from(SOURCE_DEVICE, lba, &mut sector) {
                return Err(format!("source refresh read failed lba={}", lba));
            }
            let Some(target_lba) = self.layout.root_start_lba.checked_add(lba) else {
                return Err(format!("target refresh LBA overflow lba={}", lba));
            };
            if !crate::ata::write_sector_to(self.target, target_lba, &sector) {
                return Err(format!("target refresh write failed lba={}", target_lba));
            }
            if let Some(slot) = self.root_checksums.get_mut(lba as usize) {
                *slot = sector_checksum(&sector);
            }
        }
        Ok(())
    }

    fn fail(&mut self, message: String) {
        self.phase = InstallerJobPhase::Failed;
        self.message = message;
        crate::println!(
            "[install] gui install failed target={} error={}",
            self.target.name(),
            self.message
        );
    }
}

fn compute_install_layout(
    root_sectors: u32,
    target_sectors: u32,
) -> Result<InstallLayout, &'static str> {
    let mut boot_mbr = [0u8; 512];
    if !crate::ata::read_sector_from(BOOT_DEVICE, 0, &mut boot_mbr) {
        return Err("boot MBR read failed");
    }
    let Some(partitions) = crate::disk_layout::parse_mbr(&boot_mbr) else {
        return Err("boot disk has no valid MBR");
    };
    let Some(boot_end) = crate::disk_layout::boot_area_end_lba(&partitions) else {
        return Err("boot partition layout overflow");
    };
    let Some(root_start_lba) =
        crate::disk_layout::align_up_lba(boot_end, crate::disk_layout::INSTALL_ALIGNMENT_SECTORS)
    else {
        return Err("root partition alignment overflow");
    };
    let Some(total_required_sectors) = root_start_lba.checked_add(root_sectors) else {
        return Err("target layout overflow");
    };
    if target_sectors < total_required_sectors {
        return Err("target too small");
    }
    Ok(InstallLayout {
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

fn stabilize_source_for_install() -> Result<(), &'static str> {
    crate::writeback::barrier()?;
    let _ = crate::writeback::drain(64);
    crate::writeback::barrier()
}

fn refresh_target_root_prefix(
    target: IdeDevice,
    layout: InstallLayout,
    lines: &mut Vec<String>,
) -> bool {
    if let Err(err) = stabilize_source_for_install() {
        lines.push(format!("install: source refresh failed: {}", err));
        return false;
    }
    let limit = layout.root_sectors.min(ROOT_METADATA_REFRESH_SECTORS);
    let mut sector = [0u8; 512];
    for lba in 0..limit {
        if !crate::ata::read_sector_from(SOURCE_DEVICE, lba, &mut sector) {
            lines.push(format!("install: source refresh read failed lba={}", lba));
            return false;
        }
        let Some(target_lba) = layout.root_start_lba.checked_add(lba) else {
            lines.push(format!("install: target refresh LBA overflow lba={}", lba));
            return false;
        };
        if !crate::ata::write_sector_to(target, target_lba, &sector) {
            lines.push(format!(
                "install: target refresh write failed lba={}",
                target_lba
            ));
            return false;
        }
    }
    lines.push(format!("source_refresh_sectors={}", limit));
    true
}

fn sector_checksum(sector: &[u8; 512]) -> u32 {
    let mut hash = 0x811C_9DC5u32;
    for byte in sector {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn validate_install_target(plan: &InstallPlan) -> Result<(), &'static str> {
    if plan.installable {
        Ok(())
    } else {
        Err(plan.reason)
    }
}

fn format_device_line(plan: &InstallPlan) -> String {
    let role = if plan.target == BOOT_DEVICE {
        "boot"
    } else if plan.target == SOURCE_DEVICE {
        "root"
    } else {
        "target"
    };
    format!(
        "{} present={} sectors={} role={} protected={} installable={} size_mib={} state={} reason={}",
        plan.target.name(),
        if plan.target_present { "yes" } else { "no" },
        plan.target_sectors,
        role,
        if plan.protected { "yes" } else { "no" },
        if plan.installable { "yes" } else { "no" },
        sectors_to_mib(plan.target_sectors),
        plan.state.label(),
        plan.reason,
    )
}

fn plan_lines(plan: &InstallPlan) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("plan target={}", plan.target.name()));
    lines.push(format!(
        "source_boot={} present={} sectors={}",
        plan.source_boot.name(),
        if plan.source_boot_present {
            "yes"
        } else {
            "no"
        },
        plan.source_boot_sectors
    ));
    lines.push(format!(
        "source_root={} present={} sectors={}",
        plan.source_root.name(),
        if plan.source_root_present {
            "yes"
        } else {
            "no"
        },
        plan.source_root_sectors
    ));
    lines.push(format!(
        "target={} present={} sectors={} size_mib={} state={} protected={}",
        plan.target.name(),
        if plan.target_present { "yes" } else { "no" },
        plan.target_sectors,
        sectors_to_mib(plan.target_sectors),
        plan.state.label(),
        if plan.protected { "yes" } else { "no" },
    ));
    if let Some(layout) = plan.layout {
        lines.push(format!(
            "layout=self-boot mbr_type=0x{:02x} boot_copy_sectors={} root_start_lba={} root_sectors={} required_sectors={} required_mib={}",
            crate::disk_layout::COOLFS_PARTITION_TYPE,
            layout.boot_copy_sectors,
            layout.root_start_lba,
            layout.root_sectors,
            layout.total_required_sectors,
            sectors_to_mib(layout.total_required_sectors),
        ));
    } else if plan.reason == "target too small" {
        if let Ok(layout) = compute_install_layout(plan.source_root_sectors, u32::MAX) {
            lines.push(format!(
                "layout=self-boot mbr_type=0x{:02x} boot_copy_sectors={} root_start_lba={} root_sectors={} required_sectors={} required_mib={}",
                crate::disk_layout::COOLFS_PARTITION_TYPE,
                layout.boot_copy_sectors,
                layout.root_start_lba,
                layout.root_sectors,
                layout.total_required_sectors,
                sectors_to_mib(layout.total_required_sectors),
            ));
        }
    }
    lines.push(format!(
        "installable={} reason={}",
        if plan.installable { "yes" } else { "no" },
        plan.reason
    ));
    lines
}

fn target_state(device: IdeDevice, present: bool) -> TargetState {
    if !present {
        return TargetState::Missing;
    }
    let mut sector = [0u8; 512];
    if !crate::ata::read_sector_from(device, 0, &mut sector) {
        return TargetState::Unknown;
    }
    if sector.iter().all(|&byte| byte == 0) {
        return TargetState::Blank;
    }
    if crate::disk_layout::has_coolfs_magic(&sector) {
        return TargetState::DirectCoolFs;
    }
    let Some(partitions) = crate::disk_layout::parse_mbr(&sector) else {
        return TargetState::Unknown;
    };
    if crate::disk_layout::find_partition(&partitions, crate::disk_layout::COOLFS_PARTITION_TYPE)
        .is_some()
    {
        TargetState::SelfBoot
    } else {
        TargetState::Mbr
    }
}

fn sectors_to_mib(sectors: u32) -> u32 {
    ((sectors as u64 * crate::disk_layout::SECTOR_SIZE as u64) / (1024 * 1024)) as u32
}
