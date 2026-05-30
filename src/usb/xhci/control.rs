fn build_default_control_device(
    info: &XhciInfo,
    dcbaa_virt: u64,
    slot_id: u8,
    port_num: u8,
    speed_id: u8,
) -> Result<PrimedDevice, &'static str> {
    let (input_ctx_phys, input_ctx_virt) =
        alloc_zeroed_phys().ok_or("input context alloc failed")?;
    let (output_ctx_phys, output_ctx_virt) =
        alloc_zeroed_phys().ok_or("output context alloc failed")?;
    let (transfer_ring_phys, transfer_ring_virt) =
        alloc_zeroed_phys().ok_or("control ring alloc failed")?;
    let (descriptor_phys, descriptor_virt) =
        alloc_zeroed_phys().ok_or("descriptor buffer alloc failed")?;

    unsafe {
        init_link_trb(transfer_ring_phys, CONTROL_RING_TRBS, transfer_ring_phys);
        write_u64(dcbaa_virt + slot_id as u64 * 8, output_ctx_phys);
        // Input Control Context: evaluate Slot Context (A0) and Endpoint 0 Context (A1).
        write_u32(input_ctx_virt + 0x04, 0x0000_0003);
    }

    let slot_ctx = input_ctx_virt + info.context_size as u64;
    let ep0_ctx = input_ctx_virt + (info.context_size as u64 * 2);
    let default_mps = default_control_mps(speed_id);
    if default_mps == 0 {
        return Err("unsupported default control max packet size");
    }

    unsafe {
        // Slot Context
        write_u32(slot_ctx, ((speed_id as u32) << 20) | (1 << 27));
        write_u32(slot_ctx + 0x04, (port_num as u32) << 16);

        // Endpoint 0 Context
        write_u32(ep0_ctx, 0);
        write_u32(
            ep0_ctx + 0x04,
            ((default_mps as u32) << 16) | (4 << 3) | (3 << 1),
        );
        write_u64(ep0_ctx + 0x08, transfer_ring_phys | 1);
        write_u32(ep0_ctx + 0x10, 8);
    }

    Ok(PrimedDevice {
        default_mps,
        transfer_ring: TransferRingState {
            phys: transfer_ring_phys,
            virt: transfer_ring_virt,
            enqueue_idx: 0,
            cycle: true,
            size: CONTROL_RING_TRBS,
        },
        descriptor_phys,
        descriptor_virt,
        input_ctx_phys,
        input_ctx_virt,
        output_ctx_virt,
    })
}

fn enable_slot(
    info: &XhciInfo,
    cmd_ring: &mut CommandRingState,
    event_ring: &mut EventRingState,
    slot_type: u8,
) -> Result<u8, &'static str> {
    let trb_phys = push_command_trb(
        cmd_ring,
        0,
        0,
        (TRB_TYPE_ENABLE_SLOT_CMD << 10) | ((slot_type as u32) << 16),
    );
    ring_host_doorbell(info);

    let completion = wait_for_command_completion(info, event_ring, trb_phys)?;
    if completion.completion_code != COMPLETION_SUCCESS || completion.slot_id == 0 {
        println!(
            "[xhci] enable slot failed code={} ptr={:#x} slot={}",
            completion.completion_code, completion.ptr, completion.slot_id,
        );
        return Err("enable slot failed");
    }

    println!("[xhci] enabled slot {}", completion.slot_id);
    Ok(completion.slot_id)
}

fn disable_slot(
    info: &XhciInfo,
    cmd_ring: &mut CommandRingState,
    event_ring: &mut EventRingState,
    slot_id: u8,
) -> Result<(), &'static str> {
    let trb_phys = push_command_trb(
        cmd_ring,
        0,
        0,
        (TRB_TYPE_DISABLE_SLOT_CMD << 10) | ((slot_id as u32) << 24),
    );
    ring_host_doorbell(info);

    let completion = wait_for_command_completion(info, event_ring, trb_phys)?;
    if completion.completion_code != COMPLETION_SUCCESS {
        println!(
            "[xhci] disable slot {} failed code={} ptr={:#x}",
            slot_id, completion.completion_code, completion.ptr,
        );
        return Err("disable slot failed");
    }

    Ok(())
}

fn address_device(
    info: &XhciInfo,
    cmd_ring: &mut CommandRingState,
    event_ring: &mut EventRingState,
    slot_id: u8,
    input_ctx_phys: u64,
    bsr: bool,
) -> Result<(), &'static str> {
    let trb_phys = push_command_trb(
        cmd_ring,
        input_ctx_phys,
        0,
        (TRB_TYPE_ADDRESS_DEVICE_CMD << 10) | ((bsr as u32) << 9) | ((slot_id as u32) << 24),
    );
    ring_host_doorbell(info);

    let completion = wait_for_command_completion(info, event_ring, trb_phys)?;
    if completion.completion_code != COMPLETION_SUCCESS {
        println!(
            "[xhci] address device failed slot={} code={} ptr={:#x}",
            slot_id, completion.completion_code, completion.ptr,
        );
        return Err("address device failed");
    }

    println!(
        "[xhci] slot {} address device complete bsr={}",
        slot_id, bsr as u8,
    );
    Ok(())
}

fn update_ep0_max_packet_size(info: &XhciInfo, device: &PrimedDevice, max_packet_size0: u16) {
    if max_packet_size0 == 0 {
        return;
    }

    let ep0_ctx = device.input_ctx_virt + (info.context_size as u64 * 2);
    unsafe {
        let word = read_u32(ep0_ctx + 0x04);
        write_u32(
            ep0_ctx + 0x04,
            (word & 0x0000_FFFF) | ((max_packet_size0 as u32) << 16),
        );
        write_u32(device.input_ctx_virt, 0);
        write_u32(device.input_ctx_virt + 0x04, 0x0000_0003);
    }
}

fn read_device_descriptor_header(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    slot_id: u8,
    ring: &mut TransferRingState,
    buffer_phys: u64,
    buffer_virt: u64,
) -> Result<[u8; DEVICE_DESCRIPTOR_HEADER_LEN], &'static str> {
    let descriptor_bytes = read_descriptor(
        info,
        event_ring,
        slot_id,
        ring,
        buffer_phys,
        buffer_virt,
        DESCRIPTOR_TYPE_DEVICE,
        0,
        REQUEST_RECIPIENT_DEVICE,
        0,
        DEVICE_DESCRIPTOR_HEADER_LEN,
    )?;
    let mut bytes = [0u8; DEVICE_DESCRIPTOR_HEADER_LEN];
    for (idx, byte) in bytes.iter_mut().enumerate() {
        *byte = descriptor_bytes[idx];
    }

    println!(
        "[xhci] slot {} descriptor8 {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
        slot_id, bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    );

    Ok(bytes)
}

fn read_device_descriptor(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    slot_id: u8,
    ring: &mut TransferRingState,
    buffer_phys: u64,
    buffer_virt: u64,
) -> Result<DeviceDescriptor, &'static str> {
    let bytes = read_descriptor(
        info,
        event_ring,
        slot_id,
        ring,
        buffer_phys,
        buffer_virt,
        DESCRIPTOR_TYPE_DEVICE,
        0,
        REQUEST_RECIPIENT_DEVICE,
        0,
        DEVICE_DESCRIPTOR_LEN,
    )?;

    if bytes.len() != DEVICE_DESCRIPTOR_LEN || bytes[1] != DESCRIPTOR_TYPE_DEVICE as u8 {
        return Err("malformed device descriptor");
    }

    Ok(DeviceDescriptor {
        usb_bcd: u16::from_le_bytes([bytes[2], bytes[3]]),
        class: bytes[4],
        subclass: bytes[5],
        protocol: bytes[6],
        max_packet_size0: max_packet_size0_from_descriptor(
            u16::from_le_bytes([bytes[2], bytes[3]]),
            bytes[7],
        ),
        vendor_id: u16::from_le_bytes([bytes[8], bytes[9]]),
        product_id: u16::from_le_bytes([bytes[10], bytes[11]]),
        device_bcd: u16::from_le_bytes([bytes[12], bytes[13]]),
        configurations: bytes[17],
    })
}

fn read_configuration_descriptor(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    slot_id: u8,
    ring: &mut TransferRingState,
    buffer_phys: u64,
    buffer_virt: u64,
) -> Result<Vec<u8>, &'static str> {
    let header = read_descriptor(
        info,
        event_ring,
        slot_id,
        ring,
        buffer_phys,
        buffer_virt,
        DESCRIPTOR_TYPE_CONFIGURATION,
        0,
        REQUEST_RECIPIENT_DEVICE,
        0,
        CONFIG_DESCRIPTOR_HEADER_LEN,
    )?;

    if header.len() != CONFIG_DESCRIPTOR_HEADER_LEN
        || header[1] != DESCRIPTOR_TYPE_CONFIGURATION as u8
    {
        return Err("malformed configuration descriptor header");
    }

    let total_len = u16::from_le_bytes([header[2], header[3]]) as usize;
    if !(CONFIG_DESCRIPTOR_HEADER_LEN..=DESCRIPTOR_BUFFER_BYTES).contains(&total_len) {
        return Err("configuration descriptor too large");
    }

    let config = read_descriptor(
        info,
        event_ring,
        slot_id,
        ring,
        buffer_phys,
        buffer_virt,
        DESCRIPTOR_TYPE_CONFIGURATION,
        0,
        REQUEST_RECIPIENT_DEVICE,
        0,
        total_len,
    )?;

    println!(
        "[xhci] slot {} config0 total_len={} interfaces={} max_power={}mA attrs={:#x}",
        slot_id,
        total_len,
        config[4],
        (config[8] as u16) * 2,
        config[7],
    );

    Ok(config)
}

fn set_configuration(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    slot_id: u8,
    ring: &mut TransferRingState,
    configuration_value: u8,
) -> Result<(), &'static str> {
    control_transfer_no_data(
        info,
        event_ring,
        slot_id,
        ring,
        REQUEST_TYPE_OUT | REQUEST_TYPE_STANDARD | REQUEST_RECIPIENT_DEVICE,
        SETUP_SET_CONFIGURATION,
        configuration_value as u16,
        0,
    )?;
    println!(
        "[xhci] slot {} set configuration {}",
        slot_id, configuration_value
    );
    Ok(())
}

fn set_boot_protocol(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    slot_id: u8,
    ring: &mut TransferRingState,
    interface_number: u8,
) -> Result<(), &'static str> {
    control_transfer_no_data(
        info,
        event_ring,
        slot_id,
        ring,
        REQUEST_TYPE_OUT | REQUEST_TYPE_CLASS | REQUEST_RECIPIENT_INTERFACE,
        SETUP_SET_PROTOCOL,
        0,
        interface_number as u16,
    )?;
    Ok(())
}

fn set_idle(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    slot_id: u8,
    ring: &mut TransferRingState,
    interface_number: u8,
) -> Result<(), &'static str> {
    control_transfer_no_data(
        info,
        event_ring,
        slot_id,
        ring,
        REQUEST_TYPE_OUT | REQUEST_TYPE_CLASS | REQUEST_RECIPIENT_INTERFACE,
        SETUP_SET_IDLE,
        0,
        interface_number as u16,
    )?;
    Ok(())
}

fn activate_hid_interface(
    info: &XhciInfo,
    cmd_ring: &mut CommandRingState,
    event_ring: &mut EventRingState,
    slot_id: u8,
    port_num: u8,
    speed_id: u8,
    hid: &HidInterface,
    device: &mut PrimedDevice,
) -> Result<HidDeviceState, &'static str> {
    if hid.kind == HID_KIND_KEYBOARD || hid.kind == HID_KIND_MOUSE {
        set_boot_protocol(
            info,
            event_ring,
            slot_id,
            &mut device.transfer_ring,
            hid.number,
        )?;
    }
    if hid.kind == HID_KIND_KEYBOARD || hid.kind == HID_KIND_TABLET {
        let _ = set_idle(
            info,
            event_ring,
            slot_id,
            &mut device.transfer_ring,
            hid.number,
        );
    }

    let endpoint_dci = endpoint_dci(hid.endpoint_address);
    if endpoint_dci <= CONTROL_ENDPOINT_DCI {
        return Err("invalid HID interrupt endpoint");
    }

    let (report_ring_phys, report_ring_virt) =
        alloc_zeroed_phys().ok_or("interrupt ring alloc failed")?;
    let (report_buffer_phys, report_buffer_virt) =
        alloc_zeroed_phys().ok_or("report buffer alloc failed")?;
    unsafe {
        init_link_trb(report_ring_phys, INTERRUPT_RING_TRBS, report_ring_phys);
    }

    configure_interrupt_endpoint(
        info,
        cmd_ring,
        event_ring,
        slot_id,
        speed_id,
        hid,
        endpoint_dci,
        report_ring_phys,
        device,
    )?;

    let mut hid_state = HidDeviceState {
        port_num,
        slot_id,
        kind: hid.kind,
        interface_number: hid.number,
        endpoint_address: hid.endpoint_address,
        endpoint_dci,
        report_request_len: interrupt_report_len(hid),
        report_ring: TransferRingState {
            phys: report_ring_phys,
            virt: report_ring_virt,
            enqueue_idx: 0,
            cycle: true,
            size: INTERRUPT_RING_TRBS,
        },
        report_buffer_phys,
        report_buffer_virt,
        report_trb_phys: 0,
        interval: hid.interval.max(1),
        report_count: 0,
        error_count: 0,
        last_report_len: 0,
        last_completion_code: 0,
    };
    queue_interrupt_transfer_by_base(info.db_base, &mut hid_state)?;

    Ok(hid_state)
}

fn configure_interrupt_endpoint(
    info: &XhciInfo,
    cmd_ring: &mut CommandRingState,
    event_ring: &mut EventRingState,
    slot_id: u8,
    speed_id: u8,
    hid: &HidInterface,
    endpoint_dci: u8,
    report_ring_phys: u64,
    device: &mut PrimedDevice,
) -> Result<(), &'static str> {
    unsafe {
        zero_page(device.input_ctx_virt);

        let output_slot_ctx = device.output_ctx_virt + info.context_size as u64;
        let input_slot_ctx = device.input_ctx_virt + info.context_size as u64;
        copy_context(output_slot_ctx, input_slot_ctx, info.context_size);

        let output_ep0_ctx = device.output_ctx_virt + (info.context_size as u64 * 2);
        let input_ep0_ctx = device.input_ctx_virt + (info.context_size as u64 * 2);
        copy_context(output_ep0_ctx, input_ep0_ctx, info.context_size);

        write_u32(device.input_ctx_virt, 0);
        write_u32(device.input_ctx_virt + 0x04, (1 << 0) | (1 << endpoint_dci));

        let slot_ctx_entries = read_u32(input_slot_ctx) & !(0x1F << 27);
        write_u32(
            input_slot_ctx,
            slot_ctx_entries | ((endpoint_dci.max(1) as u32) << 27),
        );

        let ep_ctx = device.input_ctx_virt + ((endpoint_dci as u64 + 1) * info.context_size as u64);
        write_u32(
            ep_ctx,
            (interrupt_interval(speed_id, hid.interval) as u32) << 16,
        );
        write_u32(
            ep_ctx + 0x04,
            ((hid.max_packet_size as u32) << 16) | (7 << 3) | (3 << 1),
        );
        write_u64(ep_ctx + 0x08, report_ring_phys | 1);
        write_u32(ep_ctx + 0x10, interrupt_report_len(hid) as u32);
    }

    let trb_phys = push_command_trb(
        cmd_ring,
        device.input_ctx_phys,
        0,
        (TRB_TYPE_CONFIGURE_ENDPOINT_CMD << 10) | ((slot_id as u32) << 24),
    );
    ring_host_doorbell(info);

    let completion = wait_for_command_completion(info, event_ring, trb_phys)?;
    if completion.completion_code != COMPLETION_SUCCESS {
        println!(
            "[xhci] configure endpoint failed slot={} code={} ptr={:#x}",
            slot_id, completion.completion_code, completion.ptr,
        );
        return Err("configure endpoint failed");
    }

    Ok(())
}

fn control_transfer_in(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    slot_id: u8,
    ring: &mut TransferRingState,
    buffer_phys: u64,
    buffer_virt: u64,
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    len: usize,
) -> Result<Vec<u8>, &'static str> {
    if len == 0 || len > DESCRIPTOR_BUFFER_BYTES || len > u16::MAX as usize {
        return Err("control transfer length unsupported");
    }

    let setup = usb_setup_packet(request_type, request, value, index, len as u16);

    let _setup_phys = push_transfer_trb(
        ring,
        setup,
        8,
        (TRB_TYPE_SETUP_STAGE << 10) | TRB_IDT | TRB_TRT_IN,
    );
    let _data_phys = push_transfer_trb(
        ring,
        buffer_phys,
        len as u32,
        (TRB_TYPE_DATA_STAGE << 10) | TRB_DIR_IN,
    );
    let status_phys = push_transfer_trb(ring, 0, 0, (TRB_TYPE_STATUS_STAGE << 10) | TRB_IOC);

    ring_device_doorbell(info, slot_id, CONTROL_ENDPOINT_DCI);

    let transfer =
        wait_for_transfer_completion(info, event_ring, slot_id, CONTROL_ENDPOINT_DCI, status_phys)?;
    if !completion_is_success_like(transfer.completion_code) {
        println!(
            "[xhci] control IN failed slot={} req={:#x} value={:#x} code={} ptr={:#x} residual={}",
            slot_id, request, value, transfer.completion_code, transfer.ptr, transfer.residual,
        );
        return Err("control IN transfer failed");
    }

    let mut bytes = Vec::with_capacity(len);
    bytes.resize(len, 0);
    for (idx, byte) in bytes.iter_mut().enumerate() {
        *byte = unsafe { core::ptr::read_volatile((buffer_virt + idx as u64) as *const u8) };
    }

    Ok(bytes)
}

fn control_transfer_no_data(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    slot_id: u8,
    ring: &mut TransferRingState,
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
) -> Result<(), &'static str> {
    let setup = usb_setup_packet(request_type, request, value, index, 0);
    let _setup_phys = push_transfer_trb(
        ring,
        setup,
        8,
        (TRB_TYPE_SETUP_STAGE << 10) | TRB_IDT | TRB_TRT_NONE,
    );
    let status_phys = push_transfer_trb(
        ring,
        0,
        0,
        (TRB_TYPE_STATUS_STAGE << 10) | TRB_IOC | TRB_DIR_IN,
    );

    ring_device_doorbell(info, slot_id, CONTROL_ENDPOINT_DCI);

    let transfer =
        wait_for_transfer_completion(info, event_ring, slot_id, CONTROL_ENDPOINT_DCI, status_phys)?;
    if !completion_is_success_like(transfer.completion_code) {
        println!(
            "[xhci] control no-data failed slot={} req={:#x} value={:#x} code={} ptr={:#x}",
            slot_id, request, value, transfer.completion_code, transfer.ptr,
        );
        return Err("control transfer failed");
    }

    Ok(())
}

fn read_descriptor(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    slot_id: u8,
    ring: &mut TransferRingState,
    buffer_phys: u64,
    buffer_virt: u64,
    descriptor_type: u16,
    descriptor_index: u8,
    recipient: u8,
    index: u16,
    len: usize,
) -> Result<Vec<u8>, &'static str> {
    control_transfer_in(
        info,
        event_ring,
        slot_id,
        ring,
        buffer_phys,
        buffer_virt,
        REQUEST_TYPE_IN | REQUEST_TYPE_STANDARD | recipient,
        SETUP_GET_DESCRIPTOR,
        (descriptor_type << 8) | descriptor_index as u16,
        index,
        len,
    )
}
