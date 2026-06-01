extern crate alloc;

use alloc::{format, string::String, vec::Vec};

use crate::ata::IdeDevice;
use crate::storage::BlockDevice;

const COOLFS_PARTITION_INDEX: usize = 2;
const GUI_INSTALL_SECTOR_BUDGET: u32 = 8192;
const ROOT_METADATA_REFRESH_SECTORS: u32 = 128;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InstallLayoutKind {
    BiosMbr,
    UefiGpt,
}

impl InstallLayoutKind {
    pub const fn label(self) -> &'static str {
        match self {
            InstallLayoutKind::BiosMbr => "self-boot",
            InstallLayoutKind::UefiGpt => "uefi-gpt",
        }
    }
}

#[derive(Clone, Copy)]
pub struct InstallLayout {
    pub kind: InstallLayoutKind,
    pub boot_start_lba: u32,
    pub boot_copy_sectors: u32,
    pub root_start_lba: u32,
    pub root_sectors: u32,
    pub total_required_sectors: u32,
    pub target_sectors: u32,
}

impl InstallLayout {
    fn source_boot_lba(self, offset: u32) -> Option<u32> {
        match self.kind {
            InstallLayoutKind::BiosMbr => Some(offset),
            InstallLayoutKind::UefiGpt => self.boot_start_lba.checked_add(offset),
        }
    }

    fn target_boot_lba(self, offset: u32) -> Option<u32> {
        match self.kind {
            InstallLayoutKind::BiosMbr => Some(offset),
            InstallLayoutKind::UefiGpt => self.boot_start_lba.checked_add(offset),
        }
    }

    fn boot_verify_start(self) -> u32 {
        match self.kind {
            InstallLayoutKind::BiosMbr => 1,
            InstallLayoutKind::UefiGpt => 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TargetState {
    Missing,
    Blank,
    DirectCoolFs,
    Mbr,
    SelfBoot,
    Gpt,
    UefiSelfBoot,
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
            TargetState::Gpt => "gpt",
            TargetState::UefiSelfBoot => "uefi-self-boot",
            TargetState::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy)]
pub struct InstallPlan {
    pub target: BlockDevice,
    pub target_present: bool,
    pub target_sectors: u32,
    pub source_boot: BlockDevice,
    pub source_boot_present: bool,
    pub source_boot_sectors: u32,
    pub source_root: BlockDevice,
    pub source_root_present: bool,
    pub source_root_base_lba: u32,
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
    PatchLayout,
    CopyRoot,
    Flush,
    VerifyLayout,
    VerifyBoot,
    VerifyRoot,
    Complete,
    Failed,
}

impl InstallerJobPhase {
    pub const fn label(self) -> &'static str {
        match self {
            InstallerJobPhase::CopyBoot => "Copying boot files",
            InstallerJobPhase::PatchLayout => "Writing installer layout",
            InstallerJobPhase::CopyRoot => "Copying coolOS root",
            InstallerJobPhase::Flush => "Flushing target disk",
            InstallerJobPhase::VerifyLayout => "Verifying boot layout",
            InstallerJobPhase::VerifyBoot => "Verifying boot files",
            InstallerJobPhase::VerifyRoot => "Verifying coolOS root",
            InstallerJobPhase::Complete => "Installation complete",
            InstallerJobPhase::Failed => "Installation failed",
        }
    }
}

pub struct InstallerJob {
    target: BlockDevice,
    layout: InstallLayout,
    phase: InstallerJobPhase,
    cursor: u32,
    completed_units: u32,
    total_units: u32,
    boot_checksums: Vec<u32>,
    root_checksums: Vec<u32>,
    message: String,
}

pub fn source_device() -> BlockDevice {
    crate::storage::root_disk()
        .map(|root| root.device)
        .unwrap_or(BlockDevice::Ide(IdeDevice::Ide0Slave))
}

pub fn boot_device() -> BlockDevice {
    let source = source_device();
    let source_info = crate::storage::device_info(source);
    if source_info.present
        && crate::storage::find_gpt_partition(
            source,
            source_info.sectors,
            crate::disk_layout::EFI_SYSTEM_PARTITION_GUID,
        )
        .is_some()
    {
        return source;
    }

    for device in crate::storage::all_devices() {
        if device == source {
            continue;
        }
        let info = crate::storage::device_info(device);
        if !info.present {
            continue;
        }
        if crate::storage::find_gpt_partition(
            device,
            info.sectors,
            crate::disk_layout::EFI_SYSTEM_PARTITION_GUID,
        )
        .is_some()
        {
            return device;
        }
    }

    let legacy_boot = BlockDevice::Ide(IdeDevice::Ide0Master);
    if crate::storage::device_info(legacy_boot).present {
        return legacy_boot;
    }
    crate::storage::all_devices()
        .into_iter()
        .find(|device| *device != source && crate::storage::device_info(*device).present)
        .unwrap_or(legacy_boot)
}

pub fn source_root_base_lba() -> u32 {
    crate::storage::root_disk()
        .map(|root| root.base_lba)
        .unwrap_or(0)
}

pub fn source_root_sectors() -> u32 {
    crate::storage::root_disk()
        .map(|root| root.sectors)
        .unwrap_or_else(|| crate::storage::device_info(source_device()).sectors)
}

fn source_root_lba(offset: u32) -> Option<u32> {
    source_root_base_lba().checked_add(offset)
}

pub fn default_target_device() -> BlockDevice {
    for candidate in preferred_target_devices() {
        if crate::storage::device_info(candidate).present && install_plan(candidate).installable {
            return candidate;
        }
    }
    for candidate in preferred_target_devices() {
        if crate::storage::device_info(candidate).present {
            return candidate;
        }
    }
    BlockDevice::Ide(IdeDevice::Ide1Master)
}

pub fn selectable_target_devices() -> Vec<BlockDevice> {
    let mut devices = Vec::new();
    for candidate in preferred_target_devices() {
        push_unique_device(&mut devices, candidate);
    }
    for device in crate::storage::all_devices() {
        push_unique_device(&mut devices, device);
    }
    devices
}

pub fn source_mode_label() -> &'static str {
    let source = source_device();
    let boot = boot_device();
    if source == boot && source.usb_index().is_some() {
        "usb-live"
    } else if source == boot {
        "single-disk"
    } else {
        "split-disk"
    }
}

pub fn device_role(device: BlockDevice) -> &'static str {
    let source = source_device();
    let boot = boot_device();
    if device == source && device == boot {
        if device.usb_index().is_some() {
            "usb-installer"
        } else {
            "boot-root"
        }
    } else if device == source {
        "root"
    } else if device == boot {
        "boot"
    } else if device.is_physical_install_target() {
        "physical-target"
    } else {
        "target"
    }
}

pub fn hardware_summary_lines() -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "installer source_mode={} source_boot={} source_root={} root_base_lba={} root_sectors={}",
        source_mode_label(),
        boot_device().name(),
        source_device().name(),
        source_root_base_lba(),
        source_root_sectors()
    ));
    for device in crate::storage::all_devices() {
        let info = crate::storage::device_info(device);
        if !info.present {
            continue;
        }
        let role = device_role(device);
        if device.is_physical_install_target() || role != "target" {
            let plan = install_plan(device);
            lines.push(format!(
                "installer candidate={} bus={} role={} protected={} installable={} reason={}",
                device.name(),
                device.bus_label(),
                role,
                if plan.protected { "yes" } else { "no" },
                if plan.installable { "yes" } else { "no" },
                plan.reason
            ));
        }
    }
    lines
}

fn preferred_target_devices() -> Vec<BlockDevice> {
    let mut devices = Vec::new();
    if source_device().usb_index().is_some() {
        for candidate in [
            BlockDevice::Nvme0n1,
            BlockDevice::Nvme1n1,
            BlockDevice::Nvme2n1,
            BlockDevice::Nvme3n1,
            BlockDevice::Sata0,
            BlockDevice::Sata1,
            BlockDevice::Sata2,
            BlockDevice::Sata3,
            BlockDevice::Sata4,
            BlockDevice::Sata5,
            BlockDevice::Sata6,
            BlockDevice::Sata7,
        ] {
            devices.push(candidate);
        }
    } else {
        for candidate in [
            BlockDevice::Nvme1n1,
            BlockDevice::Nvme0n1,
            BlockDevice::Nvme2n1,
            BlockDevice::Nvme3n1,
            BlockDevice::Sata2,
            BlockDevice::Sata3,
            BlockDevice::Ide(IdeDevice::Ide1Master),
            BlockDevice::Ide(IdeDevice::Ide1Slave),
        ] {
            devices.push(candidate);
        }
    }
    devices
}

fn push_unique_device(devices: &mut Vec<BlockDevice>, device: BlockDevice) {
    if !devices.iter().any(|existing| *existing == device) {
        devices.push(device);
    }
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
    for device in crate::storage::all_devices() {
        let plan = install_plan(device);
        lines.push(format_device_line(&plan));
    }
    lines
}

pub fn plan_device_name(name: &str) -> Vec<String> {
    let Some(target) = BlockDevice::parse(name) else {
        return alloc::vec![format!("install plan: unknown disk {}", name)];
    };
    plan_device(target)
}

pub fn plan_device(target: BlockDevice) -> Vec<String> {
    let plan = install_plan(target);
    plan_lines(&plan)
}

pub fn install_to_device_name(name: &str) -> Vec<String> {
    let Some(target) = BlockDevice::parse(name) else {
        return alloc::vec![format!("install: unknown disk {}", name)];
    };
    install_to_device(target)
}

pub fn install_physical_device_name(name: &str) -> Vec<String> {
    let Some(target) = BlockDevice::parse(name) else {
        return alloc::vec![format!("install physical: unknown disk {}", name)];
    };
    install_physical_to_device(target)
}

pub fn verify_device_name(name: &str) -> Vec<String> {
    let Some(target) = BlockDevice::parse(name) else {
        return alloc::vec![format!("install: unknown disk {}", name)];
    };
    verify_device(target)
}

pub fn install_physical_to_device(target: BlockDevice) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("install physical target={}", target.name()));
    let plan = install_plan(target);
    if plan.protected {
        lines.push(format!("install physical: {}", plan.reason));
        lines.extend(plan_lines(&plan));
        return lines;
    }
    if source_device().usb_index().is_none() || boot_device() != source_device() {
        lines.push(String::from(
            "install physical: requires a UEFI USB live source disk",
        ));
        lines.extend(plan_lines(&plan));
        return lines;
    }
    if !target.is_physical_install_target() {
        lines.push(String::from(
            "install physical: target must be internal sata* or nvme*n1",
        ));
        lines.extend(plan_lines(&plan));
        return lines;
    }
    match plan.layout.map(|layout| layout.kind) {
        Some(InstallLayoutKind::UefiGpt) => {}
        Some(InstallLayoutKind::BiosMbr) => {
            lines.push(String::from(
                "install physical: only UEFI/GPT installs are supported",
            ));
            lines.extend(plan_lines(&plan));
            return lines;
        }
        None => {
            lines.push(format!("install physical: {}", plan.reason));
            lines.extend(plan_lines(&plan));
            return lines;
        }
    }
    lines.extend(install_to_device(target));
    lines
}

pub fn review_layout_line(layout: InstallLayout) -> String {
    match layout.kind {
        InstallLayoutKind::BiosMbr => format!("BIOS/MBR + CoolFS @ LBA {}", layout.root_start_lba),
        InstallLayoutKind::UefiGpt => {
            format!("UEFI/GPT ESP + CoolFS @ LBA {}", layout.root_start_lba)
        }
    }
}

pub fn install_plan(target: BlockDevice) -> InstallPlan {
    let boot_info = crate::storage::device_info(boot_device());
    let source_info = crate::storage::device_info(source_device());
    let root_base_lba = source_root_base_lba();
    let root_sectors = source_root_sectors();
    let target_info = crate::storage::device_info(target);
    let protected = target == boot_device() || target == source_device();
    let state = target_state(target, target_info.present);

    let mut layout = None;
    let mut reason = "ready";
    let mut installable = true;

    if target == source_device() {
        reason = "refusing to overwrite mounted root disk";
        installable = false;
    } else if target == boot_device() {
        reason = "refusing to overwrite boot disk";
        installable = false;
    } else if matches!(
        target,
        BlockDevice::Ide(device)
            if !matches!(device, IdeDevice::Ide1Master | IdeDevice::Ide1Slave)
    ) {
        reason = "target must be on secondary IDE bus";
        installable = false;
    } else if !target_info.present {
        reason = "target disk not present";
        installable = false;
    } else if source_device().usb_index().is_some() && !target.is_physical_install_target() {
        reason = "physical target must be sata* or nvme*n1";
        installable = false;
    } else if !source_info.present {
        reason = "source disk not present";
        installable = false;
    } else if !boot_info.present {
        reason = "boot disk not present";
        installable = false;
    } else {
        match compute_install_layout(root_sectors, target_info.sectors) {
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
        source_root_base_lba: root_base_lba,
        source_root_sectors: root_sectors,
        layout,
        state,
        protected,
        installable,
        reason,
    }
}

pub fn install_to_device(target: BlockDevice) -> Vec<String> {
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
    lines.push(format_layout_line(layout));

    let mut sector = [0u8; 512];
    lines.push(format!(
        "copy boot started sectors={}",
        layout.boot_copy_sectors
    ));
    for lba in 0..layout.boot_copy_sectors {
        let Some(source_lba) = layout.source_boot_lba(lba) else {
            lines.push(format!("install: source boot LBA overflow offset={}", lba));
            return lines;
        };
        let Some(target_lba) = layout.target_boot_lba(lba) else {
            lines.push(format!("install: target boot LBA overflow offset={}", lba));
            return lines;
        };
        if !crate::storage::read_sector_from(boot_device(), source_lba, &mut sector) {
            lines.push(format!("install: boot read failed lba={}", source_lba));
            return lines;
        }
        if !crate::storage::write_sector_to(target, target_lba, &sector) {
            lines.push(format!(
                "install: target boot write failed lba={}",
                target_lba
            ));
            return lines;
        }
    }
    lines.push(format!(
        "copy boot complete sectors={}",
        layout.boot_copy_sectors
    ));

    if !write_target_layout(target, layout, &mut lines) {
        return lines;
    }

    lines.push(format!("copy root started sectors={}", layout.root_sectors));
    for lba in 0..layout.root_sectors {
        let Some(source_lba) = source_root_lba(lba) else {
            lines.push(format!("install: source root LBA overflow offset={}", lba));
            return lines;
        };
        if !crate::storage::read_sector_from(source_device(), source_lba, &mut sector) {
            lines.push(format!("install: source read failed lba={}", source_lba));
            return lines;
        }
        let Some(target_lba) = layout.root_start_lba.checked_add(lba) else {
            lines.push(format!("install: target LBA overflow lba={}", lba));
            return lines;
        };
        if !crate::storage::write_sector_to(target, target_lba, &sector) {
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
    if crate::storage::flush_device(target) {
        lines.push(String::from("flush=ok"));
    } else {
        lines.push(String::from("install: target flush failed"));
        return lines;
    }

    let verified = verify_self_boot_device(target, layout, &mut lines);
    if verified {
        lines.push(format!("install complete target={}", target.name()));
        lines.push(format!("reboot_with_target_as_boot_disk={}", target.name()));
    }
    lines
}

pub fn verify_device(target: BlockDevice) -> Vec<String> {
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

pub fn start_gui_install(target: BlockDevice) -> Result<InstallerJob, Vec<String>> {
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
    target: BlockDevice,
    layout: InstallLayout,
    lines: &mut Vec<String>,
) -> bool {
    lines.push(format!(
        "verify started layout={} boot_sectors={} root_sectors={}",
        layout.kind.label(),
        layout.boot_copy_sectors,
        layout.root_sectors
    ));
    if !verify_target_layout(target, layout, lines) {
        return false;
    }
    let mut source = [0u8; 512];
    let mut dest = [0u8; 512];
    for lba in layout.boot_verify_start()..layout.boot_copy_sectors {
        let Some(source_lba) = layout.source_boot_lba(lba) else {
            lines.push(format!("verify: source boot LBA overflow offset={}", lba));
            return false;
        };
        let Some(target_lba) = layout.target_boot_lba(lba) else {
            lines.push(format!("verify: target boot LBA overflow offset={}", lba));
            return false;
        };
        if !crate::storage::read_sector_from(boot_device(), source_lba, &mut source) {
            lines.push(format!("verify: boot read failed lba={}", source_lba));
            return false;
        }
        if !crate::storage::read_sector_from(target, target_lba, &mut dest) {
            lines.push(format!(
                "verify: target boot read failed lba={}",
                target_lba
            ));
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
        let Some(source_lba) = source_root_lba(lba) else {
            lines.push(format!("verify: source root LBA overflow offset={}", lba));
            return false;
        };
        if !crate::storage::read_sector_from(source_device(), source_lba, &mut source) {
            lines.push(format!("verify: source read failed lba={}", source_lba));
            return false;
        }
        let Some(target_lba) = layout.root_start_lba.checked_add(lba) else {
            lines.push(format!("verify: target LBA overflow lba={}", lba));
            return false;
        };
        if !crate::storage::read_sector_from(target, target_lba, &mut dest) {
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
        "verify=ok layout={} boot_sectors={} root_sectors={}",
        layout.kind.label(),
        layout.boot_copy_sectors,
        layout.root_sectors
    ));
    true
}

impl InstallerJob {
    fn new(target: BlockDevice, layout: InstallLayout) -> Self {
        let total_units = layout
            .boot_copy_sectors
            .saturating_add(layout.root_sectors)
            .saturating_add(1)
            .saturating_add(1)
            .saturating_add(
                layout
                    .boot_copy_sectors
                    .saturating_sub(layout.boot_verify_start()),
            )
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
                        self.phase = InstallerJobPhase::PatchLayout;
                        self.cursor = 0;
                        self.message = String::from("Boot files copied");
                        changed = true;
                        continue;
                    }
                    let lba = self.cursor;
                    let Some(source_lba) = self.layout.source_boot_lba(lba) else {
                        self.fail(format!("source boot LBA overflow offset={}", lba));
                        return true;
                    };
                    let Some(target_lba) = self.layout.target_boot_lba(lba) else {
                        self.fail(format!("target boot LBA overflow offset={}", lba));
                        return true;
                    };
                    if !crate::storage::read_sector_from(boot_device(), source_lba, &mut source) {
                        self.fail(format!("boot read failed lba={}", source_lba));
                        return true;
                    }
                    self.boot_checksums.push(sector_checksum(&source));
                    if !crate::storage::write_sector_to(self.target, target_lba, &source) {
                        self.fail(format!("target boot write failed lba={}", target_lba));
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
                InstallerJobPhase::PatchLayout => {
                    let mut lines = Vec::new();
                    if !write_target_layout(self.target, self.layout, &mut lines) {
                        let message = lines
                            .last()
                            .cloned()
                            .unwrap_or_else(|| String::from("target layout write failed"));
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
                    let Some(source_lba) = source_root_lba(lba) else {
                        self.fail(format!("source root LBA overflow offset={}", lba));
                        return true;
                    };
                    if !crate::storage::read_sector_from(source_device(), source_lba, &mut source) {
                        self.fail(format!("source read failed lba={}", source_lba));
                        return true;
                    }
                    self.root_checksums.push(sector_checksum(&source));
                    let Some(target_lba) = self.layout.root_start_lba.checked_add(lba) else {
                        self.fail(format!("target LBA overflow lba={}", lba));
                        return true;
                    };
                    if !crate::storage::write_sector_to(self.target, target_lba, &source) {
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
                    if crate::storage::flush_device(self.target) {
                        self.completed_units = self.completed_units.saturating_add(1);
                        self.phase = InstallerJobPhase::VerifyLayout;
                        self.cursor = 0;
                        self.message = String::from("Target disk flushed");
                        changed = true;
                    } else {
                        self.fail(String::from("target flush failed"));
                        return true;
                    }
                }
                InstallerJobPhase::VerifyLayout => {
                    let mut lines = Vec::new();
                    if !verify_target_layout(self.target, self.layout, &mut lines) {
                        let message = lines
                            .last()
                            .cloned()
                            .unwrap_or_else(|| String::from("target layout verify failed"));
                        self.fail(message);
                        return true;
                    }
                    self.completed_units = self.completed_units.saturating_add(1);
                    self.phase = InstallerJobPhase::VerifyBoot;
                    self.cursor = self.layout.boot_verify_start();
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
                    let Some(target_lba) = self.layout.target_boot_lba(lba) else {
                        self.fail(format!("target boot verify LBA overflow offset={}", lba));
                        return true;
                    };
                    if !crate::storage::read_sector_from(self.target, target_lba, &mut dest) {
                        self.fail(format!("target boot verify read failed lba={}", target_lba));
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
                    if !crate::storage::read_sector_from(self.target, target_lba, &mut dest) {
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
            let Some(source_lba) = source_root_lba(lba) else {
                return Err(format!("source root LBA overflow offset={}", lba));
            };
            if !crate::storage::read_sector_from(source_device(), source_lba, &mut sector) {
                return Err(format!("source refresh read failed lba={}", source_lba));
            }
            let Some(target_lba) = self.layout.root_start_lba.checked_add(lba) else {
                return Err(format!("target refresh LBA overflow lba={}", lba));
            };
            if !crate::storage::write_sector_to(self.target, target_lba, &sector) {
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
    if let Some(esp) = source_esp_partition() {
        return compute_uefi_install_layout(esp, root_sectors, target_sectors);
    }
    compute_bios_install_layout(root_sectors, target_sectors)
}

fn compute_bios_install_layout(
    root_sectors: u32,
    target_sectors: u32,
) -> Result<InstallLayout, &'static str> {
    let mut boot_mbr = [0u8; 512];
    if !crate::storage::read_sector_from(boot_device(), 0, &mut boot_mbr) {
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
        kind: InstallLayoutKind::BiosMbr,
        boot_start_lba: 0,
        boot_copy_sectors: boot_end,
        root_start_lba,
        root_sectors,
        total_required_sectors,
        target_sectors,
    })
}

fn compute_uefi_install_layout(
    esp: crate::disk_layout::GptPartition,
    root_sectors: u32,
    target_sectors: u32,
) -> Result<InstallLayout, &'static str> {
    let Some(esp_sectors) = esp.sectors() else {
        return Err("source ESP layout overflow");
    };
    let Some(esp_end) = esp.end_lba() else {
        return Err("source ESP layout overflow");
    };
    let minimum_esp_start =
        crate::disk_layout::GPT_PRIMARY_ENTRIES_LBA + crate::disk_layout::GPT_ENTRY_SECTORS;
    if esp.starting_lba < minimum_esp_start {
        return Err("source ESP overlaps GPT metadata");
    }
    let Some(root_start_lba) =
        crate::disk_layout::align_up_lba(esp_end, crate::disk_layout::INSTALL_ALIGNMENT_SECTORS)
    else {
        return Err("root partition alignment overflow");
    };
    let Some(root_end) = root_start_lba.checked_add(root_sectors) else {
        return Err("target layout overflow");
    };
    let Some(total_required_sectors) =
        root_end.checked_add(crate::disk_layout::GPT_TRAILING_SECTORS)
    else {
        return Err("target layout overflow");
    };
    if target_sectors < total_required_sectors {
        return Err("target too small");
    }
    Ok(InstallLayout {
        kind: InstallLayoutKind::UefiGpt,
        boot_start_lba: esp.starting_lba,
        boot_copy_sectors: esp_sectors,
        root_start_lba,
        root_sectors,
        total_required_sectors,
        target_sectors,
    })
}

fn source_esp_partition() -> Option<crate::disk_layout::GptPartition> {
    let boot_info = crate::storage::device_info(boot_device());
    if !boot_info.present {
        return None;
    }
    crate::storage::find_gpt_partition(
        boot_device(),
        boot_info.sectors,
        crate::disk_layout::EFI_SYSTEM_PARTITION_GUID,
    )
}

fn installed_layout_from_target(
    target: BlockDevice,
    expected_root_sectors: u32,
    lines: &mut Vec<String>,
) -> Option<InstallLayout> {
    let target_info = crate::storage::device_info(target);
    if target_info.present {
        if let Some(layout) = installed_gpt_layout_from_target(
            target,
            target_info.sectors,
            expected_root_sectors,
            lines,
        ) {
            return Some(layout);
        }
    }

    let mut target_mbr = [0u8; 512];
    if !crate::storage::read_sector_from(target, 0, &mut target_mbr) {
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
        kind: InstallLayoutKind::BiosMbr,
        boot_start_lba: 0,
        boot_copy_sectors: boot_end,
        root_start_lba: root_partition.starting_lba,
        root_sectors: root_partition.sectors,
        total_required_sectors,
        target_sectors: target_info.sectors,
    })
}

fn installed_gpt_layout_from_target(
    target: BlockDevice,
    target_sectors: u32,
    expected_root_sectors: u32,
    lines: &mut Vec<String>,
) -> Option<InstallLayout> {
    let esp = crate::storage::find_gpt_partition(
        target,
        target_sectors,
        crate::disk_layout::EFI_SYSTEM_PARTITION_GUID,
    )?;
    let root = crate::storage::find_gpt_partition(
        target,
        target_sectors,
        crate::disk_layout::COOLFS_GPT_PARTITION_GUID,
    )?;
    let Some(esp_sectors) = esp.sectors() else {
        lines.push(String::from("verify: ESP partition overflow"));
        return None;
    };
    let Some(root_sectors) = root.sectors() else {
        lines.push(String::from("verify: CoolFS GPT partition overflow"));
        return None;
    };
    if root_sectors != expected_root_sectors {
        lines.push(format!(
            "verify: root size mismatch expected={} actual={}",
            expected_root_sectors, root_sectors
        ));
        return None;
    }
    let Some(total_required_sectors) = root
        .end_lba()
        .and_then(|end| end.checked_add(crate::disk_layout::GPT_TRAILING_SECTORS))
    else {
        lines.push(String::from("verify: GPT root partition overflow"));
        return None;
    };
    Some(InstallLayout {
        kind: InstallLayoutKind::UefiGpt,
        boot_start_lba: esp.starting_lba,
        boot_copy_sectors: esp_sectors,
        root_start_lba: root.starting_lba,
        root_sectors,
        total_required_sectors,
        target_sectors,
    })
}

fn write_target_layout(
    target: BlockDevice,
    layout: InstallLayout,
    lines: &mut Vec<String>,
) -> bool {
    match layout.kind {
        InstallLayoutKind::BiosMbr => write_target_mbr(target, layout, lines),
        InstallLayoutKind::UefiGpt => write_target_gpt(target, layout, lines),
    }
}

fn verify_target_layout(
    target: BlockDevice,
    layout: InstallLayout,
    lines: &mut Vec<String>,
) -> bool {
    match layout.kind {
        InstallLayoutKind::BiosMbr => verify_target_mbr(target, layout, lines),
        InstallLayoutKind::UefiGpt => verify_target_gpt(target, layout, lines),
    }
}

fn write_target_mbr(target: BlockDevice, layout: InstallLayout, lines: &mut Vec<String>) -> bool {
    let mut mbr = [0u8; 512];
    if !crate::storage::read_sector_from(boot_device(), 0, &mut mbr) {
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
    if !crate::storage::write_sector_to(target, 0, &mbr) {
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

fn write_target_gpt(target: BlockDevice, layout: InstallLayout, lines: &mut Vec<String>) -> bool {
    let Some(root_end) = layout.root_start_lba.checked_add(layout.root_sectors) else {
        lines.push(String::from("install: GPT root partition overflow"));
        return false;
    };
    if root_end.saturating_add(crate::disk_layout::GPT_TRAILING_SECTORS) > layout.target_sectors {
        lines.push(String::from("install: GPT target too small"));
        return false;
    }
    let last_usable = layout
        .target_sectors
        .saturating_sub(crate::disk_layout::GPT_TRAILING_SECTORS);
    let backup_header_lba = layout.target_sectors.saturating_sub(1);
    let backup_entries_lba = layout
        .target_sectors
        .saturating_sub(crate::disk_layout::GPT_TRAILING_SECTORS);
    let mut entries = alloc::vec![
        0u8;
        crate::disk_layout::GPT_ENTRY_COUNT * crate::disk_layout::GPT_ENTRY_SIZE
    ];
    if crate::disk_layout::write_gpt_partition_entry(
        &mut entries,
        0,
        crate::disk_layout::EFI_SYSTEM_PARTITION_GUID,
        crate::disk_layout::INSTALL_ESP_UNIQUE_GUID,
        layout.boot_start_lba,
        layout.boot_copy_sectors,
        "coolOS EFI",
    )
    .is_none()
    {
        lines.push(String::from("install: ESP GPT entry write failed"));
        return false;
    }
    if crate::disk_layout::write_gpt_partition_entry(
        &mut entries,
        1,
        crate::disk_layout::COOLFS_GPT_PARTITION_GUID,
        crate::disk_layout::INSTALL_ROOT_UNIQUE_GUID,
        layout.root_start_lba,
        layout.root_sectors,
        "coolOS Root",
    )
    .is_none()
    {
        lines.push(String::from("install: CoolFS GPT entry write failed"));
        return false;
    }
    let entries_crc = crate::disk_layout::crc32(&entries);

    let mut sector = [0u8; 512];
    crate::disk_layout::write_protective_mbr(&mut sector, layout.target_sectors);
    if !crate::storage::write_sector_to(target, 0, &sector) {
        lines.push(String::from("install: protective MBR write failed"));
        return false;
    }

    if !write_entry_array(
        target,
        crate::disk_layout::GPT_PRIMARY_ENTRIES_LBA,
        &entries,
        lines,
        "primary",
    ) {
        return false;
    }
    if crate::disk_layout::write_gpt_header(
        &mut sector,
        crate::disk_layout::GPT_PRIMARY_HEADER_LBA,
        backup_header_lba,
        crate::disk_layout::GPT_PRIMARY_ENTRIES_LBA + crate::disk_layout::GPT_ENTRY_SECTORS,
        last_usable,
        crate::disk_layout::GPT_PRIMARY_ENTRIES_LBA,
        crate::disk_layout::GPT_ENTRY_COUNT as u32,
        entries_crc,
        crate::disk_layout::INSTALL_DISK_GUID,
    )
    .is_none()
    {
        lines.push(String::from("install: primary GPT header build failed"));
        return false;
    }
    if !crate::storage::write_sector_to(target, crate::disk_layout::GPT_PRIMARY_HEADER_LBA, &sector)
    {
        lines.push(String::from("install: primary GPT header write failed"));
        return false;
    }

    if !write_entry_array(target, backup_entries_lba, &entries, lines, "backup") {
        return false;
    }
    if crate::disk_layout::write_gpt_header(
        &mut sector,
        backup_header_lba,
        crate::disk_layout::GPT_PRIMARY_HEADER_LBA,
        crate::disk_layout::GPT_PRIMARY_ENTRIES_LBA + crate::disk_layout::GPT_ENTRY_SECTORS,
        last_usable,
        backup_entries_lba,
        crate::disk_layout::GPT_ENTRY_COUNT as u32,
        entries_crc,
        crate::disk_layout::INSTALL_DISK_GUID,
    )
    .is_none()
    {
        lines.push(String::from("install: backup GPT header build failed"));
        return false;
    }
    if !crate::storage::write_sector_to(target, backup_header_lba, &sector) {
        lines.push(String::from("install: backup GPT header write failed"));
        return false;
    }
    lines.push(format!(
        "gpt patched esp_start={} esp_sectors={} coolfs_guid={} root_start={} root_sectors={}",
        layout.boot_start_lba,
        layout.boot_copy_sectors,
        crate::disk_layout::COOLFS_GPT_PARTITION_GUID_TEXT,
        layout.root_start_lba,
        layout.root_sectors
    ));
    true
}

fn write_entry_array(
    target: BlockDevice,
    start_lba: u32,
    entries: &[u8],
    lines: &mut Vec<String>,
    label: &str,
) -> bool {
    for (idx, chunk) in entries.chunks(crate::disk_layout::SECTOR_SIZE).enumerate() {
        let Some(lba) = start_lba.checked_add(idx as u32) else {
            lines.push(format!("install: {} GPT entry LBA overflow", label));
            return false;
        };
        let mut sector = [0u8; 512];
        sector[..chunk.len()].copy_from_slice(chunk);
        if !crate::storage::write_sector_to(target, lba, &sector) {
            lines.push(format!("install: {} GPT entries write failed", label));
            return false;
        }
    }
    true
}

fn verify_target_mbr(target: BlockDevice, layout: InstallLayout, lines: &mut Vec<String>) -> bool {
    let mut source_mbr = [0u8; 512];
    let mut target_mbr = [0u8; 512];
    if !crate::storage::read_sector_from(boot_device(), 0, &mut source_mbr) {
        lines.push(String::from("verify: boot MBR read failed"));
        return false;
    }
    if !crate::storage::read_sector_from(target, 0, &mut target_mbr) {
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

fn verify_target_gpt(target: BlockDevice, layout: InstallLayout, lines: &mut Vec<String>) -> bool {
    let target_info = crate::storage::device_info(target);
    let Some(esp) = crate::storage::find_gpt_partition(
        target,
        target_info.sectors,
        crate::disk_layout::EFI_SYSTEM_PARTITION_GUID,
    ) else {
        lines.push(String::from("verify: target missing EFI system partition"));
        return false;
    };
    let Some(root) = crate::storage::find_gpt_partition(
        target,
        target_info.sectors,
        crate::disk_layout::COOLFS_GPT_PARTITION_GUID,
    ) else {
        lines.push(String::from("verify: target missing CoolFS GPT partition"));
        return false;
    };
    if esp.starting_lba != layout.boot_start_lba || esp.sectors() != Some(layout.boot_copy_sectors)
    {
        lines.push(format!(
            "verify: ESP mismatch start={} sectors={}",
            esp.starting_lba,
            esp.sectors().unwrap_or(0)
        ));
        return false;
    }
    if root.starting_lba != layout.root_start_lba || root.sectors() != Some(layout.root_sectors) {
        lines.push(format!(
            "verify: CoolFS GPT mismatch start={} sectors={}",
            root.starting_lba,
            root.sectors().unwrap_or(0)
        ));
        return false;
    }
    lines.push(String::from("verify boot=ok layout=uefi-gpt"));
    true
}

fn stabilize_source_for_install() -> Result<(), &'static str> {
    crate::writeback::barrier()?;
    let _ = crate::writeback::drain(64);
    crate::writeback::barrier()
}

fn refresh_target_root_prefix(
    target: BlockDevice,
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
        let Some(source_lba) = source_root_lba(lba) else {
            lines.push(format!("install: source root LBA overflow offset={}", lba));
            return false;
        };
        if !crate::storage::read_sector_from(source_device(), source_lba, &mut sector) {
            lines.push(format!(
                "install: source refresh read failed lba={}",
                source_lba
            ));
            return false;
        }
        let Some(target_lba) = layout.root_start_lba.checked_add(lba) else {
            lines.push(format!("install: target refresh LBA overflow lba={}", lba));
            return false;
        };
        if !crate::storage::write_sector_to(target, target_lba, &sector) {
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
    let role = device_role(plan.target);
    format!(
        "{} present={} sectors={} bus={} role={} protected={} installable={} size_mib={} state={} reason={}",
        plan.target.name(),
        if plan.target_present { "yes" } else { "no" },
        plan.target_sectors,
        plan.target.bus_label(),
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
    lines.push(format!("source_mode={}", source_mode_label()));
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
        "source_root={} present={} sectors={} base_lba={}",
        plan.source_root.name(),
        if plan.source_root_present {
            "yes"
        } else {
            "no"
        },
        plan.source_root_sectors,
        plan.source_root_base_lba
    ));
    lines.push(format!(
        "target={} present={} sectors={} bus={} role={} size_mib={} state={} protected={}",
        plan.target.name(),
        if plan.target_present { "yes" } else { "no" },
        plan.target_sectors,
        plan.target.bus_label(),
        device_role(plan.target),
        sectors_to_mib(plan.target_sectors),
        plan.state.label(),
        if plan.protected { "yes" } else { "no" },
    ));
    if let Some(layout) = plan.layout {
        lines.push(format_layout_line_with_mib(layout));
    } else if plan.reason == "target too small" {
        if let Ok(layout) = compute_install_layout(plan.source_root_sectors, u32::MAX) {
            lines.push(format_layout_line_with_mib(layout));
        }
    }
    lines.push(format!(
        "installable={} reason={}",
        if plan.installable { "yes" } else { "no" },
        plan.reason
    ));
    lines
}

fn format_layout_line(layout: InstallLayout) -> String {
    match layout.kind {
        InstallLayoutKind::BiosMbr => format!(
            "layout={} mbr_type=0x{:02x} boot_copy_sectors={} root_start_lba={} root_sectors={} required_sectors={}",
            layout.kind.label(),
            crate::disk_layout::COOLFS_PARTITION_TYPE,
            layout.boot_copy_sectors,
            layout.root_start_lba,
            layout.root_sectors,
            layout.total_required_sectors,
        ),
        InstallLayoutKind::UefiGpt => format!(
            "layout={} esp_start_lba={} esp_sectors={} coolfs_guid={} root_start_lba={} root_sectors={} required_sectors={}",
            layout.kind.label(),
            layout.boot_start_lba,
            layout.boot_copy_sectors,
            crate::disk_layout::COOLFS_GPT_PARTITION_GUID_TEXT,
            layout.root_start_lba,
            layout.root_sectors,
            layout.total_required_sectors,
        ),
    }
}

fn format_layout_line_with_mib(layout: InstallLayout) -> String {
    format!(
        "{} required_mib={}",
        format_layout_line(layout),
        sectors_to_mib(layout.total_required_sectors),
    )
}

fn target_state(device: BlockDevice, present: bool) -> TargetState {
    if !present {
        return TargetState::Missing;
    }
    let mut sector = [0u8; 512];
    if !crate::storage::read_sector_from(device, 0, &mut sector) {
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
        return TargetState::SelfBoot;
    }
    let mut gpt_header = [0u8; 512];
    if crate::storage::read_sector_from(
        device,
        crate::disk_layout::GPT_PRIMARY_HEADER_LBA,
        &mut gpt_header,
    ) && crate::disk_layout::parse_gpt_header(
        &gpt_header,
        crate::storage::device_info(device).sectors,
    )
    .is_some()
    {
        if crate::storage::find_gpt_partition(
            device,
            crate::storage::device_info(device).sectors,
            crate::disk_layout::COOLFS_GPT_PARTITION_GUID,
        )
        .is_some()
        {
            return TargetState::UefiSelfBoot;
        }
        return TargetState::Gpt;
    }
    TargetState::Mbr
}

fn sectors_to_mib(sectors: u32) -> u32 {
    ((sectors as u64 * crate::disk_layout::SECTOR_SIZE as u64) / (1024 * 1024)) as u32
}
