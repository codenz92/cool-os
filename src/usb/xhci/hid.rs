fn parse_hid_interfaces(config: &[u8]) -> Vec<HidInterface> {
    let mut interfaces = Vec::new();
    let mut current_number = 0u8;
    let mut current_alternate_setting = 0u8;
    let mut current_class = 0u8;
    let mut current_subclass = 0u8;
    let mut current_protocol = 0u8;
    let mut current_report_descriptor_len = 0u16;
    let mut offset = 0usize;

    while offset + 2 <= config.len() {
        let len = config[offset] as usize;
        if len < 2 || offset + len > config.len() {
            break;
        }

        match config[offset + 1] {
            USB_DESC_TYPE_INTERFACE if len >= 9 => {
                current_number = config[offset + 2];
                current_alternate_setting = config[offset + 3];
                current_class = config[offset + 5];
                current_subclass = config[offset + 6];
                current_protocol = config[offset + 7];
                current_report_descriptor_len = 0;
            }
            DESCRIPTOR_TYPE_HID if len >= 9 && current_class == USB_CLASS_HID => {
                let descriptor_count = config[offset + 5] as usize;
                let mut desc_off = offset + 6;
                for _ in 0..descriptor_count {
                    if desc_off + 3 > offset + len {
                        break;
                    }
                    let desc_type = config[desc_off];
                    let desc_len = u16::from_le_bytes([config[desc_off + 1], config[desc_off + 2]]);
                    if desc_type == DESCRIPTOR_TYPE_REPORT {
                        current_report_descriptor_len = desc_len;
                        break;
                    }
                    desc_off += 3;
                }
            }
            USB_DESC_TYPE_ENDPOINT if len >= 7 && current_class == USB_CLASS_HID => {
                let endpoint_address = config[offset + 2];
                let attributes = config[offset + 3] & 0x03;
                let max_packet_size =
                    u16::from_le_bytes([config[offset + 4], config[offset + 5]]) & 0x07ff;
                let kind = hid_interface_kind(
                    current_subclass,
                    current_protocol,
                    current_report_descriptor_len,
                    max_packet_size,
                );
                if endpoint_address & 0x80 != 0
                    && attributes == USB_ENDPOINT_ATTR_INTERRUPT
                    && kind != 0
                {
                    interfaces.push(HidInterface {
                        number: current_number,
                        alternate_setting: current_alternate_setting,
                        kind,
                        endpoint_address,
                        max_packet_size,
                        interval: config[offset + 6],
                        report_descriptor_len: current_report_descriptor_len,
                    });
                }
            }
            _ => {}
        }

        offset += len;
    }

    interfaces
}

fn hid_interface_kind(
    subclass: u8,
    protocol: u8,
    report_descriptor_len: u16,
    max_packet_size: u16,
) -> u8 {
    if subclass == USB_HID_SUBCLASS_BOOT && protocol == USB_HID_PROTOCOL_KEYBOARD {
        return HID_KIND_KEYBOARD;
    }
    if subclass == USB_HID_SUBCLASS_BOOT && protocol == USB_HID_PROTOCOL_MOUSE {
        return HID_KIND_MOUSE;
    }
    if protocol == 0 && report_descriptor_len == 74 && max_packet_size >= TABLET_REPORT_BYTES as u16
    {
        return HID_KIND_TABLET;
    }
    0
}

fn usb_setup_packet(request_type: u8, request: u8, value: u16, index: u16, length: u16) -> u64 {
    request_type as u64
        | ((request as u64) << 8)
        | ((value as u64) << 16)
        | ((index as u64) << 32)
        | ((length as u64) << 48)
}

fn max_packet_size0_from_descriptor(usb_bcd: u16, raw: u8) -> u16 {
    if usb_bcd >= 0x0300 {
        match raw {
            0..=15 => 1u16 << raw,
            _ => 0,
        }
    } else {
        raw as u16
    }
}

fn hid_kind_name(kind: u8) -> &'static str {
    match kind {
        HID_KIND_KEYBOARD => "keyboard",
        HID_KIND_MOUSE => "mouse",
        HID_KIND_TABLET => "tablet",
        _ => "unknown",
    }
}

fn interrupt_report_len(hid: &HidInterface) -> usize {
    match hid.kind {
        HID_KIND_KEYBOARD => BOOT_KEYBOARD_REPORT_BYTES,
        HID_KIND_MOUSE => BOOT_MOUSE_REPORT_BYTES
            .min(hid.max_packet_size as usize)
            .max(3),
        HID_KIND_TABLET => TABLET_REPORT_BYTES.min(hid.max_packet_size as usize),
        _ => hid.max_packet_size as usize,
    }
}

fn endpoint_dci(endpoint_address: u8) -> u8 {
    let ep_num = endpoint_address & 0x0f;
    if ep_num == 0 {
        CONTROL_ENDPOINT_DCI
    } else {
        ep_num * 2 + ((endpoint_address >> 7) & 0x1)
    }
}

fn interrupt_interval(speed_id: u8, interval: u8) -> u8 {
    let raw = interval.max(1);
    if speed_id >= 3 {
        raw.saturating_sub(1).min(15)
    } else {
        raw.saturating_sub(1).min(15)
    }
}

fn default_control_mps(speed_id: u8) -> u16 {
    match speed_id {
        1 | 2 => 8,
        3 => 64,
        4..=15 => 512,
        _ => 0,
    }
}
