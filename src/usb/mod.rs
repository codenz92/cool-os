/// USB subsystem (Phase 14).
///
/// Entry point for the xHCI host-controller driver.  Boot-time flow:
///
///   1. `usb::init()` scans PCI for a class-0C/subclass-03/prog-if-0x30 device
///      (xHCI).
///   2. If found, the capability registers (version, structural parameters) are
///      probed and logged.
///   3. The latest probe summary is retained so apps/commands can inspect USB
///      state without relying on the host terminal.
///   4. Later phases: controller reset, command/event ring setup, device
///      enumeration, HID class driver.
extern crate alloc;

use crate::println;
use alloc::{string::String, vec::Vec};
use spin::Mutex;

pub mod xhci;

static USB_STATUS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static LAST_INPUT_PRESENCE: Mutex<Option<(bool, bool)>> = Mutex::new(None);

pub fn status_lines() -> Vec<String> {
    let mut lines = USB_STATUS.lock().clone();
    lines.extend(xhci::runtime_status_lines());
    lines
}

pub fn input_presence() -> (bool, bool) {
    xhci::runtime_input_presence()
}

pub fn pointer_kind() -> &'static str {
    xhci::runtime_pointer_kind()
}

pub fn reconcile_input_fallbacks() {
    let current = input_presence();
    let mut last = LAST_INPUT_PRESENCE.lock();
    let previous = *last;
    if previous == Some(current) {
        return;
    }

    let (usb_keyboard, usb_mouse) = current;
    let has_8042 = crate::acpi::has_8042();
    crate::device_registry::set_usb_input(usb_keyboard, usb_mouse);
    if previous.map(|(prev_keyboard, _)| prev_keyboard) != Some(usb_keyboard) {
        if usb_keyboard {
            println!("[input] USB keyboard detected; PS/2 keyboard fallback disabled");
            crate::notifications::push_transient("USB input", "keyboard detected");
            crate::keyboard::disable_ps2_fallback();
        } else if has_8042 {
            println!("[input] no USB keyboard detected; enabling PS/2 keyboard fallback");
            crate::notifications::push_transient("USB input", "keyboard fallback enabled");
            crate::keyboard::enable_ps2_fallback();
        } else {
            println!("[input] no USB keyboard detected; no PS/2 controller reported by ACPI");
        }
    }

    if previous.map(|(_, prev_mouse)| prev_mouse) != Some(usb_mouse) {
        if usb_mouse {
            println!("[input] USB mouse detected; PS/2 mouse fallback disabled");
            crate::notifications::push_transient("USB input", "mouse detected");
            crate::mouse::disable_ps2_fallback();
        } else if has_8042 {
            println!("[input] no USB mouse detected; enabling PS/2 mouse fallback");
            crate::notifications::push_transient("USB input", "mouse fallback enabled");
            crate::mouse::enable_ps2_fallback();
        } else {
            println!("[input] no USB mouse detected; no PS/2 controller reported by ACPI");
        }
    }

    *last = Some(current);
}

/// Called once from `kernel_main` after the VMM and interrupt controller are up.
/// Logs a line on success; silently does nothing if no xHCI controller is present
/// (the PS/2 path still owns input).
pub fn init() {
    *USB_STATUS.lock() = xhci::probe();
    reconcile_input_fallbacks();
}

pub fn poll() {
    xhci::poll();
    reconcile_input_fallbacks();
}
