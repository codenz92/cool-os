/// xHCI host controller driver — Phase 14.
///
/// Full active init runs on every boot: ownership handoff, reset, DCBAA,
/// scratchpad buffers, command ring, event ring, device enumeration, and
/// HID boot-protocol interrupt transfers. PS/2 fallback is kept alive only
/// when ACPI FADT reports an 8042 controller and no USB device takes over.
extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};
use core::sync::atomic::{fence, Ordering};
use spin::Mutex;

use crate::pci::{self, Header, Location};
use crate::println;

const PCI_CLASS_SERIAL: u8 = 0x0C;
const PCI_SUBCLASS_USB: u8 = 0x03;
const PCI_PROGIF_XHCI: u8 = 0x30;

const CAP_HCSPARAMS1: u64 = 0x04;
const CAP_HCSPARAMS2: u64 = 0x08;
const CAP_HCCPARAMS1: u64 = 0x10;
const CAP_DBOFF: u64 = 0x14;
const CAP_RTSOFF: u64 = 0x18;

const OP_USBCMD: u64 = 0x00;
const OP_USBSTS: u64 = 0x04;
const OP_PAGESIZE: u64 = 0x08;
const OP_CRCR: u64 = 0x18;
const OP_DCBAAP: u64 = 0x30;
const OP_CONFIG: u64 = 0x38;
const OP_PORTSC_BASE: u64 = 0x400;

const RT_IR0: u64 = 0x20;
const IR0_ERSTSZ: u64 = 0x08;
const IR0_ERSTBA: u64 = 0x10;
const IR0_ERDP: u64 = 0x18;

const USBCMD_RS: u32 = 1 << 0;
const USBCMD_HCRST: u32 = 1 << 1;

const USBSTS_HCH: u32 = 1 << 0;
const USBSTS_CNR: u32 = 1 << 11;

const PORTSC_CCS: u32 = 1 << 0;
const PORTSC_PED: u32 = 1 << 1;
const PORTSC_PR: u32 = 1 << 4;
const PORTSC_PP: u32 = 1 << 9;
const PORTSC_SPEED_SHIFT: u32 = 10;
const PORTSC_SPEED_MASK: u32 = 0xF << PORTSC_SPEED_SHIFT;
const PORTSC_CSC: u32 = 1 << 17;
const PORTSC_PEC: u32 = 1 << 18;
const PORTSC_WRC: u32 = 1 << 19;
const PORTSC_OCC: u32 = 1 << 20;
const PORTSC_PRC: u32 = 1 << 21;
const PORTSC_PLC: u32 = 1 << 22;
const PORTSC_CEC: u32 = 1 << 23;
const PORTSC_CHANGE_BITS: u32 =
    PORTSC_CSC | PORTSC_PEC | PORTSC_WRC | PORTSC_OCC | PORTSC_PRC | PORTSC_PLC | PORTSC_CEC;

const EXT_CAP_LEGACY_SUPPORT: u8 = 1;
const EXT_CAP_SUPPORTED_PROTOCOL: u8 = 2;
const EXT_CAP_EXT_POWER_MGMT: u8 = 3;
const EXT_CAP_IO_VIRT: u8 = 4;
const EXT_CAP_MSG_INTERRUPT: u8 = 5;
const EXT_CAP_USB_DEBUG: u8 = 10;
const EXT_CAP_EXT_MSG_INTERRUPT: u8 = 17;

const COMMAND_RING_TRBS: usize = 256;
const CONTROL_RING_TRBS: usize = 256;
const EVENT_RING_TRBS: usize = 256;
const INTERRUPT_RING_TRBS: usize = 256;
const TRB_TYPE_NORMAL: u32 = 1;
const TRB_TYPE_SETUP_STAGE: u32 = 2;
const TRB_TYPE_DATA_STAGE: u32 = 3;
const TRB_TYPE_STATUS_STAGE: u32 = 4;
const TRB_TYPE_LINK: u32 = 6;
const TRB_TYPE_ENABLE_SLOT_CMD: u32 = 9;
const TRB_TYPE_DISABLE_SLOT_CMD: u32 = 10;
const TRB_TYPE_ADDRESS_DEVICE_CMD: u32 = 11;
const TRB_TYPE_CONFIGURE_ENDPOINT_CMD: u32 = 12;
const TRB_TYPE_NOOP_CMD: u32 = 23;
const TRB_TYPE_TRANSFER_EVENT: u32 = 32;
const TRB_TYPE_CMD_COMPLETION: u32 = 33;
const TRB_TYPE_PORT_STATUS_CHANGE: u32 = 34;
const TRB_TC: u32 = 1 << 1;
const TRB_CYCLE: u32 = 1 << 0;
const TRB_IOC: u32 = 1 << 5;
const TRB_IDT: u32 = 1 << 6;
const TRB_DIR_IN: u32 = 1 << 16;
const TRB_TRT_NONE: u32 = 0 << 16;
const TRB_TRT_IN: u32 = 3 << 16;
const COMPLETION_SUCCESS: u8 = 1;
const COMPLETION_SHORT_PACKET: u8 = 13;
const ERDP_EHB_CLEAR: u64 = 1 << 3;
const CONTROL_ENDPOINT_DCI: u8 = 1;
const SETUP_GET_DESCRIPTOR: u8 = 6;
const SETUP_SET_CONFIGURATION: u8 = 9;
const SETUP_SET_IDLE: u8 = 10;
const SETUP_SET_PROTOCOL: u8 = 11;
const REQUEST_TYPE_IN: u8 = 0x80;
const REQUEST_TYPE_OUT: u8 = 0x00;
const REQUEST_TYPE_STANDARD: u8 = 0x00;
const REQUEST_TYPE_CLASS: u8 = 0x20;
const REQUEST_RECIPIENT_DEVICE: u8 = 0x00;
const REQUEST_RECIPIENT_INTERFACE: u8 = 0x01;
const DESCRIPTOR_TYPE_DEVICE: u16 = 1;
const DESCRIPTOR_TYPE_CONFIGURATION: u16 = 2;
const DESCRIPTOR_TYPE_HID: u8 = 0x21;
const DESCRIPTOR_TYPE_REPORT: u8 = 0x22;
const DEVICE_DESCRIPTOR_HEADER_LEN: usize = 8;
const DEVICE_DESCRIPTOR_LEN: usize = 18;
const CONFIG_DESCRIPTOR_HEADER_LEN: usize = 9;
const USB_DESC_TYPE_INTERFACE: u8 = 0x04;
const USB_DESC_TYPE_ENDPOINT: u8 = 0x05;
const USB_ENDPOINT_ATTR_INTERRUPT: u8 = 0x03;
const USB_CLASS_HID: u8 = 0x03;
const USB_HID_SUBCLASS_BOOT: u8 = 0x01;
const USB_HID_PROTOCOL_KEYBOARD: u8 = 0x01;
const USB_HID_PROTOCOL_MOUSE: u8 = 0x02;
const HID_KIND_KEYBOARD: u8 = 1;
const HID_KIND_MOUSE: u8 = 2;
const HID_KIND_TABLET: u8 = 3;
const DESCRIPTOR_BUFFER_BYTES: usize = 4096;
const BOOT_KEYBOARD_REPORT_BYTES: usize = 8;
const BOOT_MOUSE_REPORT_BYTES: usize = 4;
const TABLET_REPORT_BYTES: usize = 6;

const SPIN_TIMEOUT: u64 = 10_000_000;

#[derive(Clone)]
struct LegacySupport {
    off: u64,
}

#[derive(Clone)]
struct ProtocolSpeedId {
    psiv: u8,
    psie: u8,
    plt: u8,
    pfd: bool,
    lp: u8,
    psim: u16,
}

#[derive(Clone)]
struct SupportedProtocol {
    label: &'static str,
    major: u8,
    minor: u8,
    port_offset: u8,
    port_count: u8,
    psi_count: u8,
    slot_type: u8,
    psis: Vec<ProtocolSpeedId>,
}

#[derive(Clone)]
struct XhciInfo {
    mmio_virt: u64,
    caplength: u8,
    version: u16,
    max_slots: u8,
    max_interrupters: u16,
    max_ports: u8,
    scratchpad_count: u32,
    ac64: bool,
    xecp: u64,
    context_size: usize,
    op_base: u64,
    rt_base: u64,
    db_base: u64,
    legacy: Option<LegacySupport>,
    protocols: Vec<SupportedProtocol>,
}

struct ActiveState {
    info: XhciInfo,
    rt_base: u64,
    db_base: u64,
    dcbaa_phys: u64,
    dcbaa_virt: u64,
    cmd_ring_phys: u64,
    cmd_ring: CommandRingState,
    event_ring_phys: u64,
    event_ring: EventRingState,
    erst_phys: u64,
    devices: Vec<HidDeviceState>,
    poll_count: u64,
    event_count: u64,
    last_runtime_note: String,
    port_status: Vec<String>,
}

struct CommandRingState {
    phys: u64,
    virt: u64,
    enqueue_idx: usize,
    cycle: bool,
}

struct TransferRingState {
    phys: u64,
    virt: u64,
    enqueue_idx: usize,
    cycle: bool,
    size: usize,
}

struct EventRingState {
    phys: u64,
    virt: u64,
    dequeue_idx: usize,
    cycle: bool,
}

struct EventTrb {
    parameter: u64,
    status: u32,
    control: u32,
}

impl EventTrb {
    fn trb_type(&self) -> u8 {
        ((self.control >> 10) & 0x3F) as u8
    }

    fn completion_code(&self) -> u8 {
        (self.status >> 24) as u8
    }

    fn slot_id(&self) -> u8 {
        (self.control >> 24) as u8
    }

    fn endpoint_id(&self) -> u8 {
        ((self.control >> 16) & 0x1F) as u8
    }

    fn port_id(&self) -> u8 {
        (self.parameter >> 24) as u8
    }

    fn residual(&self) -> u32 {
        self.status & 0x00FF_FFFF
    }
}

struct CommandCompletion {
    ptr: u64,
    completion_code: u8,
    slot_id: u8,
}

struct TransferCompletion {
    ptr: u64,
    completion_code: u8,
    slot_id: u8,
    endpoint_id: u8,
    residual: u32,
}

struct PrimedDevice {
    default_mps: u16,
    transfer_ring: TransferRingState,
    descriptor_phys: u64,
    descriptor_virt: u64,
    input_ctx_phys: u64,
    input_ctx_virt: u64,
    output_ctx_virt: u64,
}

struct DeviceDescriptor {
    usb_bcd: u16,
    class: u8,
    subclass: u8,
    protocol: u8,
    max_packet_size0: u16,
    vendor_id: u16,
    product_id: u16,
    device_bcd: u16,
    configurations: u8,
}

struct HidInterface {
    number: u8,
    alternate_setting: u8,
    kind: u8,
    endpoint_address: u8,
    max_packet_size: u16,
    interval: u8,
    report_descriptor_len: u16,
}

struct HidDeviceState {
    port_num: u8,
    slot_id: u8,
    kind: u8,
    interface_number: u8,
    endpoint_address: u8,
    endpoint_dci: u8,
    report_request_len: usize,
    report_ring: TransferRingState,
    report_buffer_phys: u64,
    report_buffer_virt: u64,
    report_trb_phys: u64,
    interval: u8,
    report_count: u64,
    error_count: u64,
    last_report_len: usize,
    last_completion_code: u8,
}

static RUNTIME: Mutex<Option<ActiveState>> = Mutex::new(None);

pub fn probe() -> Vec<String> {
    *RUNTIME.lock() = None;
    let mut status = Vec::new();
    let mut runtime_started = false;

    let Some((loc, hdr, mmio_phys)) = find_controller() else {
        println!("[xhci] no controller found on PCI bus");
        status.push(String::from("USB: no xHCI controller found"));
        return status;
    };

    println!(
        "[xhci] {:04x}:{:02x}.{} vendor={:04x} device={:04x} mmio={:#x}",
        loc.bus, loc.device, loc.function, hdr.vendor_id, hdr.device_id, mmio_phys,
    );
    status.push(format!(
        "USB: xHCI {:04x}:{:02x}.{} vendor={:04x} device={:04x}",
        loc.bus, loc.device, loc.function, hdr.vendor_id, hdr.device_id,
    ));

    pci::enable_bus_master(loc);

    let mmio_virt = crate::vmm::phys_to_virt(x86_64::PhysAddr::new(mmio_phys)).as_u64();
    let info = read_info(mmio_virt);

    println!(
        "[xhci] version=0x{:04x} caplength={} op={:#x} rt={:#x} db={:#x}",
        info.version, info.caplength, info.op_base, info.rt_base, info.db_base,
    );
    println!(
        "[xhci] slots={} interrupters={} ports={} scratchpads={} 64bit={} xecp={:#x}",
        info.max_slots,
        info.max_interrupters,
        info.max_ports,
        info.scratchpad_count,
        info.ac64,
        info.xecp,
    );
    status.push(format!(
        "USB: xHCI v0x{:04x}, slots={}, ports={}, scratchpads={}, 64bit={}",
        info.version, info.max_slots, info.max_ports, info.scratchpad_count, info.ac64 as u8,
    ));

    match active_init(&info) {
        Ok(state) => {
            println!(
                "[xhci] active init ready dcbaa={:#x} cmd={:#x} evt={:#x} erst={:#x}",
                state.dcbaa_phys, state.cmd_ring_phys, state.event_ring_phys, state.erst_phys,
            );
            status.push(String::from("USB: active init ready"));
            *RUNTIME.lock() = Some(state);
            runtime_started = true;
        }
        Err(err) => {
            println!(
                "[xhci] active init failed: {}; falling back to passive scan",
                err
            );
            status.push(format!("USB: active init failed: {}", err));
        }
    }

    if !runtime_started {
        status.extend(scan_ports(&info));
    }
    status
}

pub fn poll() {
    let mut runtime_guard = RUNTIME.lock();
    let Some(runtime) = runtime_guard.as_mut() else {
        return;
    };

    runtime.poll_count = runtime.poll_count.saturating_add(1);
    while let Some(event) = next_event_by_base(runtime.rt_base, &mut runtime.event_ring) {
        runtime.event_count = runtime.event_count.saturating_add(1);
        match event.trb_type() as u32 {
            TRB_TYPE_TRANSFER_EVENT => handle_runtime_transfer_event(runtime, event),
            TRB_TYPE_PORT_STATUS_CHANGE => {
                handle_runtime_port_status_change(runtime, event.port_id());
            }
            TRB_TYPE_CMD_COMPLETION => {
                runtime.last_runtime_note = format!(
                    "unexpected command completion slot={} code={}",
                    event.slot_id(),
                    event.completion_code(),
                );
                println!(
                    "[xhci] runtime event: unexpected command completion ptr={:#x} code={} slot={}",
                    event.parameter & !0xFu64,
                    event.completion_code(),
                    event.slot_id(),
                );
            }
            _ => {
                runtime.last_runtime_note = format!(
                    "event type={} code={}",
                    event.trb_type(),
                    event.completion_code(),
                );
                println!(
                    "[xhci] runtime event: type={} code={} param={:#x} status={:#x}",
                    event.trb_type(),
                    event.completion_code(),
                    event.parameter,
                    event.status,
                );
            }
        }
    }
}

pub fn runtime_status_lines() -> Vec<String> {
    let runtime_guard = RUNTIME.lock();
    let Some(runtime) = runtime_guard.as_ref() else {
        return Vec::new();
    };

    let mut lines = Vec::new();
    lines.push(format!(
        "USB: runtime devices={} polls={} events={}",
        runtime.devices.len(),
        runtime.poll_count,
        runtime.event_count,
    ));
    if !runtime.last_runtime_note.is_empty() {
        lines.push(format!("USB: runtime {}", runtime.last_runtime_note));
    }
    lines.extend(runtime.port_status.iter().cloned());
    for device in runtime.devices.iter() {
        lines.push(format!(
            "USB: runtime port={} slot={} {} ep={:#04x} reports={} last={}B errors={} cc={}",
            device.port_num,
            device.slot_id,
            hid_kind_name(device.kind),
            device.endpoint_address,
            device.report_count,
            device.last_report_len,
            device.error_count,
            device.last_completion_code,
        ));
    }
    lines
}

pub fn runtime_input_presence() -> (bool, bool) {
    let runtime_guard = RUNTIME.lock();
    let Some(runtime) = runtime_guard.as_ref() else {
        return (false, false);
    };

    let mut keyboard = false;
    let mut mouse = false;
    for device in runtime.devices.iter() {
        if device.kind == HID_KIND_KEYBOARD {
            keyboard = true;
        } else if device.kind == HID_KIND_MOUSE || device.kind == HID_KIND_TABLET {
            mouse = true;
        }
    }

    (keyboard, mouse)
}

pub fn runtime_pointer_kind() -> &'static str {
    let runtime_guard = RUNTIME.lock();
    let Some(runtime) = runtime_guard.as_ref() else {
        return "none";
    };

    let mut has_mouse = false;
    for device in runtime.devices.iter() {
        if device.kind == HID_KIND_TABLET {
            return "tablet";
        }
        if device.kind == HID_KIND_MOUSE {
            has_mouse = true;
        }
    }

    if has_mouse {
        "mouse"
    } else {
        "none"
    }
}

// Section files are included into this module so the split stays behavior-neutral.

include!("init.rs");
include!("ports.rs");
include!("control.rs");
include!("hid.rs");
include!("rings.rs");
include!("runtime.rs");
include!("discovery.rs");
