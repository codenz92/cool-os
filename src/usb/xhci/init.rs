fn read_info(mmio_virt: u64) -> XhciInfo {
    let cap_word = unsafe { read_u32(mmio_virt) };
    let caplength = (cap_word & 0xFF) as u8;
    let version = (cap_word >> 16) as u16;
    let hcsparams1 = unsafe { read_u32(mmio_virt + CAP_HCSPARAMS1) };
    let hcsparams2 = unsafe { read_u32(mmio_virt + CAP_HCSPARAMS2) };
    let hccparams1 = unsafe { read_u32(mmio_virt + CAP_HCCPARAMS1) };
    let xecp = ((hccparams1 >> 16) & 0xFFFF) as u64 * 4;
    let op_base = mmio_virt + caplength as u64;
    let rt_base = mmio_virt + (unsafe { read_u32(mmio_virt + CAP_RTSOFF) } as u64 & !0x1F);
    let db_base = mmio_virt + (unsafe { read_u32(mmio_virt + CAP_DBOFF) } as u64 & !0x3);

    let max_slots = (hcsparams1 & 0xFF) as u8;
    let max_interrupters = ((hcsparams1 >> 8) & 0x7FF) as u16;
    let max_ports = ((hcsparams1 >> 24) & 0xFF) as u8;
    let scratch_hi = (hcsparams2 >> 21) & 0x1F;
    let scratch_lo = (hcsparams2 >> 27) & 0x1F;
    let scratchpad_count = (scratch_hi << 5) | scratch_lo;
    let ac64 = hccparams1 & 0x1 != 0;
    let context_size = if hccparams1 & (1 << 2) != 0 { 64 } else { 32 };
    let (legacy, protocols) = scan_extended_caps(mmio_virt, xecp);

    XhciInfo {
        mmio_virt,
        caplength,
        version,
        max_slots,
        max_interrupters,
        max_ports,
        scratchpad_count,
        ac64,
        xecp,
        context_size,
        op_base,
        rt_base,
        db_base,
        legacy,
        protocols,
    }
}

fn active_init(info: &XhciInfo) -> Result<ActiveState, &'static str> {
    let pagesize = unsafe { read_u32(info.op_base + OP_PAGESIZE) };
    if pagesize & 0x1 == 0 {
        return Err("4KiB pages not supported by controller");
    }

    if let Some(legacy) = &info.legacy {
        request_handoff(info.mmio_virt, legacy.off)?;
    }

    stop_controller(info.op_base)?;
    reset_controller(info.op_base)?;

    unsafe {
        let cfg = read_u32(info.op_base + OP_CONFIG);
        write_u32(
            info.op_base + OP_CONFIG,
            (cfg & !0xFF) | info.max_slots as u32,
        );
    }

    let (dcbaa_phys, dcbaa_virt) = alloc_zeroed_phys().ok_or("dcbaa alloc failed")?;

    // Scratchpad buffers: one 4 KiB page per entry, pointers written into a
    // page-aligned array whose physical address goes in DCBAA slot 0.
    if info.scratchpad_count > 512 {
        return Err("scratchpad count exceeds one-page array capacity");
    }
    if info.scratchpad_count > 0 {
        let (sb_array_phys, sb_array_virt) =
            alloc_zeroed_phys().ok_or("scratchpad array alloc failed")?;
        unsafe {
            write_u64(dcbaa_virt, sb_array_phys);
        }
        for i in 0..info.scratchpad_count {
            let (sb_phys, _) = alloc_zeroed_phys().ok_or("scratchpad page alloc failed")?;
            unsafe {
                write_u64(sb_array_virt + i as u64 * 8, sb_phys);
            }
        }
        println!(
            "[xhci] allocated {} scratchpad pages",
            info.scratchpad_count
        );
    }

    let (cmd_ring_phys, cmd_ring_virt) = alloc_zeroed_phys().ok_or("command ring alloc failed")?;
    let (event_ring_phys, event_ring_virt) =
        alloc_zeroed_phys().ok_or("event ring alloc failed")?;
    let (erst_phys, erst_virt) = alloc_zeroed_phys().ok_or("erst alloc failed")?;
    let mut cmd_ring = CommandRingState {
        phys: cmd_ring_phys,
        virt: cmd_ring_virt,
        enqueue_idx: 0,
        cycle: true,
    };
    let mut event_ring = EventRingState {
        phys: event_ring_phys,
        virt: event_ring_virt,
        dequeue_idx: 0,
        cycle: true,
    };

    unsafe {
        init_link_trb(cmd_ring.phys, COMMAND_RING_TRBS, cmd_ring.phys);

        write_u64(info.op_base + OP_DCBAAP, dcbaa_phys);
        write_u64(info.op_base + OP_CRCR, cmd_ring.phys | 0x1);

        write_u64(erst_virt, event_ring.phys);
        write_u32(erst_virt + 8, EVENT_RING_TRBS as u32);
        write_u32(erst_virt + 12, 0);

        let ir0 = info.rt_base + RT_IR0;
        write_u32(ir0 + IR0_ERSTSZ, 1);
        write_u64(ir0 + IR0_ERSTBA, erst_phys);
        write_u64(ir0 + IR0_ERDP, event_ring.phys);
    }

    unsafe {
        let cmd = read_u32(info.op_base + OP_USBCMD);
        write_u32(info.op_base + OP_USBCMD, cmd | USBCMD_RS);
    }
    wait_until("controller start", || unsafe {
        read_u32(info.op_base + OP_USBSTS) & USBSTS_HCH == 0
    })?;
    run_command_ring_noop(info, &mut cmd_ring, &mut event_ring)?;
    let (port_status, devices) =
        prime_attached_ports(info, dcbaa_virt, &mut cmd_ring, &mut event_ring);

    Ok(ActiveState {
        info: info.clone(),
        rt_base: info.rt_base,
        db_base: info.db_base,
        dcbaa_phys,
        dcbaa_virt,
        cmd_ring_phys,
        cmd_ring,
        event_ring_phys,
        event_ring,
        erst_phys,
        devices,
        poll_count: 0,
        event_count: 0,
        last_runtime_note: String::from("polling ready"),
        port_status,
    })
}

fn request_handoff(mmio_virt: u64, off: u64) -> Result<(), &'static str> {
    let addr = mmio_virt + off;
    let header = unsafe { read_u32(addr) };
    let bios_owned = (header >> 16) & 0x1 != 0;
    if !bios_owned {
        return Ok(());
    }

    unsafe {
        write_u32(addr, header | (1 << 24));
    }
    wait_until("bios handoff", || unsafe {
        read_u32(addr) & (1 << 16) == 0
    })
}

fn stop_controller(op_base: u64) -> Result<(), &'static str> {
    let cmd = unsafe { read_u32(op_base + OP_USBCMD) };
    if cmd & USBCMD_RS == 0 {
        return Ok(());
    }

    unsafe {
        write_u32(op_base + OP_USBCMD, cmd & !USBCMD_RS);
    }
    wait_until("controller halt", || unsafe {
        read_u32(op_base + OP_USBSTS) & USBSTS_HCH != 0
    })
}

fn reset_controller(op_base: u64) -> Result<(), &'static str> {
    wait_until("controller ready", || unsafe {
        read_u32(op_base + OP_USBSTS) & USBSTS_CNR == 0
    })?;

    unsafe {
        let cmd = read_u32(op_base + OP_USBCMD);
        write_u32(op_base + OP_USBCMD, cmd | USBCMD_HCRST);
    }

    wait_until("reset complete", || unsafe {
        let cmd = read_u32(op_base + OP_USBCMD);
        let sts = read_u32(op_base + OP_USBSTS);
        cmd & USBCMD_HCRST == 0 && sts & USBSTS_CNR == 0
    })
}

fn wait_until<F: Fn() -> bool>(label: &'static str, ready: F) -> Result<(), &'static str> {
    for _ in 0..SPIN_TIMEOUT {
        if ready() {
            return Ok(());
        }
        core::hint::spin_loop();
    }
    println!("[xhci] timeout while waiting for {}", label);
    Err(label)
}

fn alloc_zeroed_phys() -> Option<(u64, u64)> {
    let frame = crate::vmm::alloc_zeroed_frame()?;
    let phys = frame.start_address().as_u64();
    let virt = crate::vmm::phys_to_virt(frame.start_address()).as_u64();
    Some((phys, virt))
}
