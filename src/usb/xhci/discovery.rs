fn find_controller() -> Option<(Location, Header, u64)> {
    let mut found: Option<(Location, Header, u64)> = None;
    pci::scan(|loc, hdr| {
        if found.is_some() {
            return;
        }
        if hdr.class == PCI_CLASS_SERIAL
            && hdr.subclass == PCI_SUBCLASS_USB
            && hdr.prog_if == PCI_PROGIF_XHCI
        {
            if let Some(base) = pci::bar(loc, 0) {
                found = Some((loc, hdr, base));
            }
        }
    });
    found
}

unsafe fn read_u32(addr: u64) -> u32 {
    core::ptr::read_volatile(addr as *const u32)
}

unsafe fn read_u64(addr: u64) -> u64 {
    core::ptr::read_volatile(addr as *const u64)
}

unsafe fn write_u32(addr: u64, val: u32) {
    core::ptr::write_volatile(addr as *mut u32, val)
}

unsafe fn write_u64(addr: u64, val: u64) {
    core::ptr::write_volatile(addr as *mut u64, val)
}

unsafe fn zero_page(addr: u64) {
    core::ptr::write_bytes(addr as *mut u8, 0, 4096);
}

unsafe fn copy_context(src: u64, dst: u64, len: usize) {
    core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, len);
}

fn scan_extended_caps(base: u64, mut off: u64) -> (Option<LegacySupport>, Vec<SupportedProtocol>) {
    let mut legacy = None;
    let mut protocols = Vec::new();

    if off == 0 {
        println!("[xhci] no extended capabilities");
        return (legacy, protocols);
    }

    for _ in 0..32 {
        let header = unsafe { read_u32(base + off) };
        let cap_id = (header & 0xFF) as u8;
        let next = ((header >> 8) & 0xFF) as u64 * 4;

        match cap_id {
            EXT_CAP_LEGACY_SUPPORT => {
                legacy = Some(log_legacy_support(base, off, header));
            }
            EXT_CAP_SUPPORTED_PROTOCOL => {
                protocols.push(log_supported_protocol(base, off, header));
            }
            EXT_CAP_EXT_POWER_MGMT => {
                println!("[xhci] ext cap @+{:#x}: extended power management", off);
            }
            EXT_CAP_IO_VIRT => {
                println!("[xhci] ext cap @+{:#x}: I/O virtualization", off);
            }
            EXT_CAP_MSG_INTERRUPT => {
                println!("[xhci] ext cap @+{:#x}: message interrupt", off);
            }
            EXT_CAP_USB_DEBUG => {
                println!("[xhci] ext cap @+{:#x}: USB debug capability", off);
            }
            EXT_CAP_EXT_MSG_INTERRUPT => {
                println!("[xhci] ext cap @+{:#x}: extended message interrupt", off);
            }
            0 => {
                println!("[xhci] ext cap @+{:#x}: invalid id=0", off);
            }
            _ => {
                println!(
                    "[xhci] ext cap @+{:#x}: id={} header={:#x}",
                    off, cap_id, header
                );
            }
        }

        if next == 0 {
            if legacy.is_none() {
                println!("[xhci] no USB legacy support capability");
            }
            return (legacy, protocols);
        }
        off += next;
    }

    if legacy.is_none() {
        println!("[xhci] no USB legacy support capability");
    }
    println!("[xhci] extended capability scan truncated");
    (legacy, protocols)
}

fn log_supported_protocol(base: u64, off: u64, header: u32) -> SupportedProtocol {
    let name = unsafe { read_u32(base + off + 0x04) };
    let ports = unsafe { read_u32(base + off + 0x08) };
    let slot = unsafe { read_u32(base + off + 0x0C) };

    let rev_minor = ((header >> 16) & 0xFF) as u8;
    let rev_major = ((header >> 24) & 0xFF) as u8;
    let port_offset = (ports & 0xFF) as u8;
    let port_count = ((ports >> 8) & 0xFF) as u8;
    let psi_count = ((ports >> 28) & 0xF) as u8;
    let slot_type = (slot & 0x1F) as u8;
    let label = protocol_label(protocol_name(name), rev_major, rev_minor);
    let port_last = port_offset.saturating_add(port_count.saturating_sub(1));
    let mut psis = Vec::new();

    println!(
        "[xhci] ext cap @+{:#x}: supported protocol {} rev={}.{} ports={}..{} psic={} slot_type={}",
        off,
        label,
        rev_major,
        bcd_hex(rev_minor),
        port_offset,
        port_last,
        psi_count,
        slot_type,
    );

    for idx in 0..psi_count {
        let psi = unsafe { read_u32(base + off + 0x10 + idx as u64 * 4) };
        psis.push(log_psi(idx, psi));
    }

    SupportedProtocol {
        label,
        major: rev_major,
        minor: rev_minor,
        port_offset,
        port_count,
        psi_count,
        slot_type,
        psis,
    }
}

fn log_legacy_support(base: u64, off: u64, header: u32) -> LegacySupport {
    let ctlsts = unsafe { read_u32(base + off + 0x04) };
    let bios_owned = (header >> 16) & 0x1 != 0;
    let os_owned = (header >> 24) & 0x1 != 0;
    let smi_usb_en = ctlsts & (1 << 0) != 0;
    let smi_hse_en = ctlsts & (1 << 4) != 0;
    let smi_os_own_en = ctlsts & (1 << 13) != 0;
    let smi_pci_cmd_en = ctlsts & (1 << 14) != 0;
    let smi_bar_en = ctlsts & (1 << 15) != 0;
    let smi_usb = ctlsts & (1 << 16) != 0;
    let smi_hse = ctlsts & (1 << 20) != 0;
    let smi_os_own = ctlsts & (1 << 29) != 0;
    let smi_pci_cmd = ctlsts & (1 << 30) != 0;
    let smi_bar = ctlsts & (1 << 31) != 0;

    println!(
        "[xhci] ext cap @+{:#x}: USB legacy support bios_owned={} os_owned={} ctlsts={:#x}",
        off, bios_owned as u8, os_owned as u8, ctlsts,
    );

    if smi_usb_en
        || smi_hse_en
        || smi_os_own_en
        || smi_pci_cmd_en
        || smi_bar_en
        || smi_usb
        || smi_hse
        || smi_os_own
        || smi_pci_cmd
        || smi_bar
    {
        println!(
            "[xhci]   legacy smi en usb={} hse={} own={} pci={} bar={} pending usb={} hse={} own={} pci={} bar={}",
            smi_usb_en as u8,
            smi_hse_en as u8,
            smi_os_own_en as u8,
            smi_pci_cmd_en as u8,
            smi_bar_en as u8,
            smi_usb as u8,
            smi_hse as u8,
            smi_os_own as u8,
            smi_pci_cmd as u8,
            smi_bar as u8,
        );
    }

    LegacySupport { off }
}

fn scan_ports(info: &XhciInfo) -> Vec<String> {
    let mut status = Vec::new();
    let mut any = false;
    for port in 0..info.max_ports {
        let port_num = port + 1;
        let portsc = unsafe { read_u32(info.op_base + OP_PORTSC_BASE + 0x10 * port as u64) };
        let connected = portsc & PORTSC_CCS != 0;
        let enabled = portsc & PORTSC_PED != 0;
        let speed_id = ((portsc >> 10) & 0xF) as u8;

        if !connected && !enabled {
            continue;
        }

        any = true;
        if let Some(proto) = protocol_for_port(&info.protocols, port_num) {
            println!(
                "[xhci] port {} proto={} rev={}.{} slot_type={} ccs={} ped={} speed_id={} speed={} portsc={:#x}",
                port_num,
                proto.label,
                proto.major,
                bcd_hex(proto.minor),
                proto.slot_type,
                connected as u8,
                enabled as u8,
                speed_id,
                port_speed_name(proto, speed_id),
                portsc,
            );
            status.push(format!(
                "USB: port {} {} connected={} enabled={} speed={}",
                port_num,
                proto.label,
                connected as u8,
                enabled as u8,
                port_speed_name(proto, speed_id),
            ));
        } else {
            println!(
                "[xhci] port {} proto=? ccs={} ped={} speed_id={} portsc={:#x}",
                port_num, connected as u8, enabled as u8, speed_id, portsc,
            );
            status.push(format!(
                "USB: port {} connected={} enabled={} speed_id={}",
                port_num, connected as u8, enabled as u8, speed_id,
            ));
        }
    }

    if !any {
        println!("[xhci] no active root-hub ports reported");
        status.push(String::from("USB: no active root-hub ports reported"));
    }
    status
}

fn protocol_for_port(protocols: &[SupportedProtocol], port_num: u8) -> Option<&SupportedProtocol> {
    protocols.iter().find(|proto| {
        let start = proto.port_offset;
        let end = proto
            .port_offset
            .saturating_add(proto.port_count.saturating_sub(1));
        port_num >= start && port_num <= end
    })
}

fn port_speed_name(proto: &SupportedProtocol, speed_id: u8) -> String {
    if proto.psi_count != 0 {
        if let Some(psi) = proto.psis.iter().find(|psi| psi.psiv == speed_id) {
            return speed_name_from_psi(psi);
        }
        return String::from("?");
    }

    match (proto.major, proto.minor, speed_id) {
        (2, _, 1) => String::from("Full"),
        (2, _, 2) => String::from("Low"),
        (2, _, 3) => String::from("High"),
        (3, 0x00, 4) => String::from("Super"),
        (3, 0x10, 4) => String::from("Super"),
        (3, 0x10, 5) => String::from("Super+"),
        (3, 0x20, 4) => String::from("Super"),
        (3, 0x20, 5) => String::from("Super+ Gen2x1"),
        (3, 0x20, 6) => String::from("Super+ Gen1x2"),
        (3, 0x20, 7) => String::from("Super+ Gen2x2"),
        _ => String::from("?"),
    }
}

fn protocol_name(raw: u32) -> [u8; 4] {
    raw.to_le_bytes()
}

fn protocol_label(name: [u8; 4], major: u8, minor: u8) -> &'static str {
    if name == *b"USB " {
        match (major, minor) {
            (2, 0x00) => "USB 2.0",
            (3, 0x00) => "USB 3.0",
            (3, 0x10) => "USB 3.1",
            (3, 0x20) => "USB 3.2",
            _ => "USB",
        }
    } else {
        "unknown"
    }
}

fn bcd_hex(v: u8) -> u8 {
    ((v >> 4) * 10) + (v & 0x0F)
}

fn log_psi(idx: u8, psi: u32) -> ProtocolSpeedId {
    let parsed = ProtocolSpeedId {
        psiv: (psi & 0x0F) as u8,
        psie: ((psi >> 4) & 0x03) as u8,
        plt: ((psi >> 6) & 0x03) as u8,
        pfd: ((psi >> 8) & 0x01) != 0,
        lp: ((psi >> 14) & 0x03) as u8,
        psim: ((psi >> 16) & 0xFFFF) as u16,
    };

    println!(
        "[xhci]   psi{}: id={} rate={} {} kind={} duplex={} link={} raw={:#x}",
        idx,
        parsed.psiv,
        parsed.psim,
        psi_units(parsed.psie),
        psi_type(parsed.plt),
        if parsed.pfd { "full" } else { "half" },
        link_protocol(parsed.lp),
        psi,
    );

    parsed
}

fn psi_units(psie: u8) -> &'static str {
    match psie {
        0 => "b/s",
        1 => "Kb/s",
        2 => "Mb/s",
        3 => "Gb/s",
        _ => "?",
    }
}

fn psi_type(plt: u8) -> &'static str {
    match plt {
        0 => "sym",
        2 => "rx",
        3 => "tx",
        _ => "?",
    }
}

fn link_protocol(lp: u8) -> &'static str {
    match lp {
        0 => "SS",
        1 => "SSP",
        _ => "?",
    }
}

fn speed_name_from_psi(psi: &ProtocolSpeedId) -> String {
    format!(
        "{} {} {} {}",
        psi.psim,
        psi_units(psi.psie),
        psi_type(psi.plt),
        if psi.pfd { "full" } else { "half" },
    )
}

unsafe fn init_link_trb(ring_phys: u64, ring_size: usize, target_phys: u64) {
    let last_trb = crate::vmm::phys_to_virt(x86_64::PhysAddr::new(ring_phys)).as_u64()
        + ((ring_size - 1) * 16) as u64;
    write_u32(last_trb, (target_phys & 0xFFFF_FFFF) as u32);
    write_u32(last_trb + 4, (target_phys >> 32) as u32);
    write_u32(last_trb + 8, 0);
    write_link_trb_cycle(
        crate::vmm::phys_to_virt(x86_64::PhysAddr::new(ring_phys)).as_u64(),
        ring_size,
        true,
    );
}

unsafe fn write_link_trb_cycle(ring_virt: u64, ring_size: usize, cycle: bool) {
    let last_trb = ring_virt + ((ring_size - 1) * 16) as u64;
    // Link TRBs are owned by cycle bit too; refresh this before each ring wrap.
    let control = (TRB_TYPE_LINK << 10) | TRB_TC | (if cycle { TRB_CYCLE } else { 0 });
    write_u32(last_trb + 12, control);
}
