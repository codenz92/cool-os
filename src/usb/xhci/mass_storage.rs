const MSC_CBW_SIGNATURE: u32 = 0x4342_5355;
const MSC_CSW_SIGNATURE: u32 = 0x5342_5355;
const MSC_CBW_LEN: usize = 31;
const MSC_CSW_LEN: usize = 13;
const MSC_CBW_OFF: u64 = 0;
const MSC_DATA_OFF: u64 = 512;
const MSC_CSW_OFF: u64 = 3584;
const MSC_MAX_DATA_BYTES: usize = (MSC_CSW_OFF - MSC_DATA_OFF) as usize;
const MSC_CSW_STATUS_OK: u8 = 0;
const MSC_CSW_STATUS_FAILED: u8 = 1;

#[derive(Clone, Copy, PartialEq, Eq)]
enum BotDirection {
    None,
    In,
    Out,
}

fn parse_msc_interfaces(config: &[u8]) -> Vec<MscInterface> {
    let mut interfaces = Vec::new();
    let mut current_number = 0u8;
    let mut current_alternate_setting = 0u8;
    let mut current_class = 0u8;
    let mut current_subclass = 0u8;
    let mut current_protocol = 0u8;
    let mut bulk_in: Option<BulkEndpoint> = None;
    let mut bulk_out: Option<BulkEndpoint> = None;
    let mut offset = 0usize;

    while offset + 2 <= config.len() {
        let len = config[offset] as usize;
        if len < 2 || offset + len > config.len() {
            break;
        }

        match config[offset + 1] {
            USB_DESC_TYPE_INTERFACE if len >= 9 => {
                push_msc_interface(
                    &mut interfaces,
                    current_number,
                    current_alternate_setting,
                    current_class,
                    current_subclass,
                    current_protocol,
                    bulk_in,
                    bulk_out,
                );
                current_number = config[offset + 2];
                current_alternate_setting = config[offset + 3];
                current_class = config[offset + 5];
                current_subclass = config[offset + 6];
                current_protocol = config[offset + 7];
                bulk_in = None;
                bulk_out = None;
            }
            USB_DESC_TYPE_ENDPOINT if len >= 7 && current_class == USB_CLASS_MASS_STORAGE => {
                let endpoint_address = config[offset + 2];
                let attributes = config[offset + 3] & 0x03;
                if attributes == USB_ENDPOINT_ATTR_BULK {
                    let endpoint = BulkEndpoint {
                        address: endpoint_address,
                        dci: endpoint_dci(endpoint_address),
                        max_packet_size: u16::from_le_bytes([config[offset + 4], config[offset + 5]])
                            & 0x07ff,
                    };
                    if endpoint.address & 0x80 != 0 {
                        bulk_in = Some(endpoint);
                    } else {
                        bulk_out = Some(endpoint);
                    }
                }
            }
            _ => {}
        }

        offset += len;
    }

    push_msc_interface(
        &mut interfaces,
        current_number,
        current_alternate_setting,
        current_class,
        current_subclass,
        current_protocol,
        bulk_in,
        bulk_out,
    );

    interfaces
}

fn push_msc_interface(
    interfaces: &mut Vec<MscInterface>,
    number: u8,
    alternate_setting: u8,
    class: u8,
    subclass: u8,
    protocol: u8,
    bulk_in: Option<BulkEndpoint>,
    bulk_out: Option<BulkEndpoint>,
) {
    if class != USB_CLASS_MASS_STORAGE
        || subclass != USB_MSC_SUBCLASS_SCSI
        || protocol != USB_MSC_PROTOCOL_BULK_ONLY
    {
        return;
    }
    let (Some(bulk_in), Some(bulk_out)) = (bulk_in, bulk_out) else {
        return;
    };
    if bulk_in.dci <= CONTROL_ENDPOINT_DCI || bulk_out.dci <= CONTROL_ENDPOINT_DCI {
        return;
    }
    interfaces.push(MscInterface {
        number,
        alternate_setting,
        bulk_in,
        bulk_out,
    });
}

fn activate_msc_interface(
    info: &XhciInfo,
    cmd_ring: &mut CommandRingState,
    event_ring: &mut EventRingState,
    slot_id: u8,
    port_num: u8,
    msc: &MscInterface,
    device: &mut PrimedDevice,
) -> Result<UsbStorageDeviceState, &'static str> {
    let index = next_usb_storage_index()?;
    let (bulk_in_ring_phys, bulk_in_ring_virt) =
        alloc_zeroed_phys().ok_or("bulk in ring alloc failed")?;
    let (bulk_out_ring_phys, bulk_out_ring_virt) =
        alloc_zeroed_phys().ok_or("bulk out ring alloc failed")?;
    let (buffer_phys, buffer_virt) = alloc_zeroed_phys().ok_or("msc buffer alloc failed")?;
    unsafe {
        init_link_trb(bulk_in_ring_phys, BULK_RING_TRBS, bulk_in_ring_phys);
        init_link_trb(bulk_out_ring_phys, BULK_RING_TRBS, bulk_out_ring_phys);
    }

    configure_bulk_endpoints(
        info,
        cmd_ring,
        event_ring,
        slot_id,
        msc,
        bulk_in_ring_phys,
        bulk_out_ring_phys,
        device,
    )?;

    let mut state = UsbStorageDeviceState {
        index,
        port_num,
        slot_id,
        interface_number: msc.number,
        bulk_in: msc.bulk_in,
        bulk_out: msc.bulk_out,
        control_ring: device.transfer_ring,
        bulk_in_ring: TransferRingState {
            phys: bulk_in_ring_phys,
            virt: bulk_in_ring_virt,
            enqueue_idx: 0,
            cycle: true,
            size: BULK_RING_TRBS,
        },
        bulk_out_ring: TransferRingState {
            phys: bulk_out_ring_phys,
            virt: bulk_out_ring_virt,
            enqueue_idx: 0,
            cycle: true,
            size: BULK_RING_TRBS,
        },
        buffer_phys,
        buffer_virt,
        sectors: 0,
        block_size: 0,
        tag: 1,
        transfer_count: 0,
        error_count: 0,
        last_status: 0,
    };

    init_scsi_disk(info, event_ring, &mut state)?;
    Ok(state)
}

fn configure_bulk_endpoints(
    info: &XhciInfo,
    cmd_ring: &mut CommandRingState,
    event_ring: &mut EventRingState,
    slot_id: u8,
    msc: &MscInterface,
    bulk_in_ring_phys: u64,
    bulk_out_ring_phys: u64,
    device: &mut PrimedDevice,
) -> Result<(), &'static str> {
    let max_dci = msc.bulk_in.dci.max(msc.bulk_out.dci);
    unsafe {
        zero_page(device.input_ctx_virt);

        let output_slot_ctx = device.output_ctx_virt + info.context_size as u64;
        let input_slot_ctx = device.input_ctx_virt + info.context_size as u64;
        copy_context(output_slot_ctx, input_slot_ctx, info.context_size);

        let output_ep0_ctx = device.output_ctx_virt + (info.context_size as u64 * 2);
        let input_ep0_ctx = device.input_ctx_virt + (info.context_size as u64 * 2);
        copy_context(output_ep0_ctx, input_ep0_ctx, info.context_size);

        write_u32(device.input_ctx_virt, 0);
        write_u32(
            device.input_ctx_virt + 0x04,
            (1 << 0) | (1 << msc.bulk_in.dci) | (1 << msc.bulk_out.dci),
        );

        let slot_ctx_entries = read_u32(input_slot_ctx) & !(0x1F << 27);
        write_u32(input_slot_ctx, slot_ctx_entries | ((max_dci as u32) << 27));

        write_bulk_endpoint_context(info, device, msc.bulk_in, bulk_in_ring_phys);
        write_bulk_endpoint_context(info, device, msc.bulk_out, bulk_out_ring_phys);
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
            "[xhci] configure bulk endpoints failed slot={} code={} ptr={:#x}",
            slot_id, completion.completion_code, completion.ptr,
        );
        return Err("configure bulk endpoints failed");
    }

    Ok(())
}

unsafe fn write_bulk_endpoint_context(
    info: &XhciInfo,
    device: &PrimedDevice,
    endpoint: BulkEndpoint,
    ring_phys: u64,
) {
    let ep_ctx = device.input_ctx_virt + ((endpoint.dci as u64 + 1) * info.context_size as u64);
    let ep_type = if endpoint.address & 0x80 != 0 { 6 } else { 2 };
    write_u32(ep_ctx, 0);
    write_u32(
        ep_ctx + 0x04,
        ((endpoint.max_packet_size as u32) << 16) | (ep_type << 3) | (3 << 1),
    );
    write_u64(ep_ctx + 0x08, ring_phys | 1);
    write_u32(ep_ctx + 0x10, 512);
}

fn init_scsi_disk(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    device: &mut UsbStorageDeviceState,
) -> Result<(), &'static str> {
    let _ = scsi_inquiry(info, event_ring, device);
    for _ in 0..8 {
        if scsi_test_unit_ready(info, event_ring, device).is_ok() {
            break;
        }
        let _ = scsi_request_sense(info, event_ring, device);
    }
    let (sectors, block_size) = scsi_read_capacity(info, event_ring, device)?;
    if sectors == 0 || block_size == 0 {
        return Err("msc disk has no capacity");
    }
    device.sectors = sectors;
    device.block_size = block_size;
    Ok(())
}

fn scsi_inquiry(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    device: &mut UsbStorageDeviceState,
) -> Result<(), &'static str> {
    let cdb = [0x12, 0, 0, 0, 36, 0];
    bot_command(info, event_ring, device, &cdb, BotDirection::In, 36).map(|_| ())
}

fn scsi_test_unit_ready(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    device: &mut UsbStorageDeviceState,
) -> Result<(), &'static str> {
    let cdb = [0x00, 0, 0, 0, 0, 0];
    bot_command(info, event_ring, device, &cdb, BotDirection::None, 0).map(|_| ())
}

fn scsi_request_sense(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    device: &mut UsbStorageDeviceState,
) -> Result<(), &'static str> {
    let cdb = [0x03, 0, 0, 0, 18, 0];
    bot_command(info, event_ring, device, &cdb, BotDirection::In, 18).map(|_| ())
}

fn scsi_read_capacity(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    device: &mut UsbStorageDeviceState,
) -> Result<(u32, u32), &'static str> {
    let cdb = [0x25, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    bot_command(info, event_ring, device, &cdb, BotDirection::In, 8)?;
    let last_lba = read_be_u32(device.buffer_virt + MSC_DATA_OFF);
    let block_size = read_be_u32(device.buffer_virt + MSC_DATA_OFF + 4);
    Ok((last_lba.saturating_add(1), block_size))
}

fn scsi_read_sector(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    device: &mut UsbStorageDeviceState,
    lba: u32,
    buf: &mut [u8; 512],
) -> Result<(), &'static str> {
    if device.block_size != 512 || lba >= device.sectors {
        return Err("msc read out of range");
    }
    let cdb = read_write_10_cdb(0x28, lba);
    bot_command(info, event_ring, device, &cdb, BotDirection::In, 512)?;
    for (idx, byte) in buf.iter_mut().enumerate() {
        *byte =
            unsafe { core::ptr::read_volatile((device.buffer_virt + MSC_DATA_OFF + idx as u64) as *const u8) };
    }
    Ok(())
}

fn scsi_write_sector(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    device: &mut UsbStorageDeviceState,
    lba: u32,
    buf: &[u8; 512],
) -> Result<(), &'static str> {
    if device.block_size != 512 || lba >= device.sectors {
        return Err("msc write out of range");
    }
    for (idx, byte) in buf.iter().enumerate() {
        unsafe {
            core::ptr::write_volatile(
                (device.buffer_virt + MSC_DATA_OFF + idx as u64) as *mut u8,
                *byte,
            );
        }
    }
    let cdb = read_write_10_cdb(0x2A, lba);
    bot_command(info, event_ring, device, &cdb, BotDirection::Out, 512).map(|_| ())
}

fn scsi_flush(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    device: &mut UsbStorageDeviceState,
) -> Result<(), &'static str> {
    let cdb = [0x35, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    bot_command(info, event_ring, device, &cdb, BotDirection::None, 0).map(|_| ())
}

fn read_write_10_cdb(opcode: u8, lba: u32) -> [u8; 10] {
    [
        opcode,
        0,
        (lba >> 24) as u8,
        (lba >> 16) as u8,
        (lba >> 8) as u8,
        lba as u8,
        0,
        0,
        1,
        0,
    ]
}

fn bot_command(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    device: &mut UsbStorageDeviceState,
    cdb: &[u8],
    direction: BotDirection,
    data_len: usize,
) -> Result<usize, &'static str> {
    if cdb.is_empty() || cdb.len() > 16 || data_len > MSC_MAX_DATA_BYTES {
        return Err("msc command unsupported length");
    }

    let tag = device.tag;
    device.tag = device.tag.wrapping_add(1).max(1);
    if direction == BotDirection::Out {
        clear_msc_control_areas(device.buffer_virt);
    } else {
        clear_msc_buffer(device.buffer_virt);
    }
    write_cbw(device.buffer_virt + MSC_CBW_OFF, tag, cdb, direction, data_len);

    bulk_transfer(
        info,
        event_ring,
        device,
        false,
        device.buffer_phys + MSC_CBW_OFF,
        MSC_CBW_LEN,
    )?;

    let mut actual_len = 0usize;
    if data_len > 0 {
        actual_len = bulk_transfer(
            info,
            event_ring,
            device,
            direction == BotDirection::In,
            device.buffer_phys + MSC_DATA_OFF,
            data_len,
        )?;
    }

    bulk_transfer(
        info,
        event_ring,
        device,
        true,
        device.buffer_phys + MSC_CSW_OFF,
        MSC_CSW_LEN,
    )?;

    let sig = read_le_u32(device.buffer_virt + MSC_CSW_OFF);
    let csw_tag = read_le_u32(device.buffer_virt + MSC_CSW_OFF + 4);
    let status = unsafe { core::ptr::read_volatile((device.buffer_virt + MSC_CSW_OFF + 12) as *const u8) };
    device.last_status = status;
    if sig != MSC_CSW_SIGNATURE || csw_tag != tag {
        device.error_count = device.error_count.saturating_add(1);
        recover_bot(info, event_ring, device);
        return Err("msc invalid csw");
    }
    if status == MSC_CSW_STATUS_OK {
        device.transfer_count = device.transfer_count.saturating_add(1);
        return Ok(actual_len);
    }
    device.error_count = device.error_count.saturating_add(1);
    if status == MSC_CSW_STATUS_FAILED {
        return Err("msc command failed");
    }
    recover_bot(info, event_ring, device);
    Err("msc phase error")
}

fn bulk_transfer(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    device: &mut UsbStorageDeviceState,
    direction_in: bool,
    buffer_phys: u64,
    len: usize,
) -> Result<usize, &'static str> {
    if len == 0 || len > MSC_TRANSFER_BYTES {
        return Err("bulk transfer length unsupported");
    }
    let (ring, dci) = if direction_in {
        (&mut device.bulk_in_ring, device.bulk_in.dci)
    } else {
        (&mut device.bulk_out_ring, device.bulk_out.dci)
    };
    let dir_flag = if direction_in { TRB_DIR_IN } else { 0 };
    let trb_phys = push_transfer_trb(
        ring,
        buffer_phys,
        len as u32,
        (TRB_TYPE_NORMAL << 10) | TRB_IOC | dir_flag,
    );
    ring_device_doorbell(info, device.slot_id, dci);
    let completion = wait_for_transfer_completion(info, event_ring, device.slot_id, dci, trb_phys)?;
    if !completion_is_success_like(completion.completion_code) {
        device.error_count = device.error_count.saturating_add(1);
        return Err("bulk transfer failed");
    }
    Ok(len.saturating_sub(completion.residual as usize))
}

fn recover_bot(info: &XhciInfo, event_ring: &mut EventRingState, device: &mut UsbStorageDeviceState) {
    let slot_id = device.slot_id;
    let interface_number = device.interface_number as u16;
    let bulk_in = device.bulk_in.address as u16;
    let bulk_out = device.bulk_out.address as u16;

    let _ = control_transfer_no_data(
        info,
        event_ring,
        slot_id,
        &mut device.control_ring,
        REQUEST_TYPE_OUT | REQUEST_TYPE_CLASS | REQUEST_RECIPIENT_INTERFACE,
        SETUP_MSC_BOT_RESET,
        0,
        interface_number,
    );
    let _ = control_transfer_no_data(
        info,
        event_ring,
        slot_id,
        &mut device.control_ring,
        REQUEST_TYPE_OUT | REQUEST_TYPE_STANDARD | REQUEST_RECIPIENT_ENDPOINT,
        SETUP_CLEAR_FEATURE,
        USB_FEATURE_ENDPOINT_HALT,
        bulk_in,
    );
    let _ = control_transfer_no_data(
        info,
        event_ring,
        slot_id,
        &mut device.control_ring,
        REQUEST_TYPE_OUT | REQUEST_TYPE_STANDARD | REQUEST_RECIPIENT_ENDPOINT,
        SETUP_CLEAR_FEATURE,
        USB_FEATURE_ENDPOINT_HALT,
        bulk_out,
    );
}

fn write_cbw(virt: u64, tag: u32, cdb: &[u8], direction: BotDirection, data_len: usize) {
    write_le_u32(virt, MSC_CBW_SIGNATURE);
    write_le_u32(virt + 4, tag);
    write_le_u32(virt + 8, data_len as u32);
    unsafe {
        core::ptr::write_volatile(
            (virt + 12) as *mut u8,
            if direction == BotDirection::In { 0x80 } else { 0x00 },
        );
        core::ptr::write_volatile((virt + 13) as *mut u8, 0);
        core::ptr::write_volatile((virt + 14) as *mut u8, cdb.len() as u8);
        for idx in 0..16 {
            let byte = cdb.get(idx).copied().unwrap_or(0);
            core::ptr::write_volatile((virt + 15 + idx as u64) as *mut u8, byte);
        }
    }
}

fn clear_msc_buffer(virt: u64) {
    for idx in 0..MSC_TRANSFER_BYTES {
        unsafe {
            core::ptr::write_volatile((virt + idx as u64) as *mut u8, 0);
        }
    }
}

fn clear_msc_control_areas(virt: u64) {
    for idx in 0..MSC_DATA_OFF as usize {
        unsafe {
            core::ptr::write_volatile((virt + idx as u64) as *mut u8, 0);
        }
    }
    for idx in 0..MSC_CSW_LEN {
        unsafe {
            core::ptr::write_volatile((virt + MSC_CSW_OFF + idx as u64) as *mut u8, 0);
        }
    }
}

fn read_be_u32(virt: u64) -> u32 {
    let b0 = unsafe { core::ptr::read_volatile(virt as *const u8) } as u32;
    let b1 = unsafe { core::ptr::read_volatile((virt + 1) as *const u8) } as u32;
    let b2 = unsafe { core::ptr::read_volatile((virt + 2) as *const u8) } as u32;
    let b3 = unsafe { core::ptr::read_volatile((virt + 3) as *const u8) } as u32;
    (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
}

fn read_le_u32(virt: u64) -> u32 {
    let b0 = unsafe { core::ptr::read_volatile(virt as *const u8) } as u32;
    let b1 = unsafe { core::ptr::read_volatile((virt + 1) as *const u8) } as u32;
    let b2 = unsafe { core::ptr::read_volatile((virt + 2) as *const u8) } as u32;
    let b3 = unsafe { core::ptr::read_volatile((virt + 3) as *const u8) } as u32;
    b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
}

fn write_le_u32(virt: u64, value: u32) {
    unsafe {
        core::ptr::write_volatile(virt as *mut u8, value as u8);
        core::ptr::write_volatile((virt + 1) as *mut u8, (value >> 8) as u8);
        core::ptr::write_volatile((virt + 2) as *mut u8, (value >> 16) as u8);
        core::ptr::write_volatile((virt + 3) as *mut u8, (value >> 24) as u8);
    }
}

fn next_usb_storage_index() -> Result<u8, &'static str> {
    let index = NEXT_STORAGE_INDEX.fetch_add(1, Ordering::Relaxed);
    if index < MSC_MAX_DEVICES as u8 {
        Ok(index)
    } else {
        Err("too many usb storage devices")
    }
}

fn block_device_for_usb_index(index: u8) -> Option<crate::storage::BlockDevice> {
    match index {
        0 => Some(crate::storage::BlockDevice::Usb0),
        1 => Some(crate::storage::BlockDevice::Usb1),
        2 => Some(crate::storage::BlockDevice::Usb2),
        3 => Some(crate::storage::BlockDevice::Usb3),
        4 => Some(crate::storage::BlockDevice::Usb4),
        5 => Some(crate::storage::BlockDevice::Usb5),
        6 => Some(crate::storage::BlockDevice::Usb6),
        7 => Some(crate::storage::BlockDevice::Usb7),
        _ => None,
    }
}

pub fn storage_devices() -> Vec<crate::storage::BlockDevice> {
    let runtime_guard = RUNTIME.lock();
    let Some(runtime) = runtime_guard.as_ref() else {
        return Vec::new();
    };
    runtime
        .storage_devices
        .iter()
        .filter_map(|device| block_device_for_usb_index(device.index))
        .collect()
}

pub fn storage_device_info(device: crate::storage::BlockDevice) -> crate::storage::BlockDeviceInfo {
    let Some(index) = device.usb_index() else {
        return crate::storage::BlockDeviceInfo {
            device,
            present: false,
            sectors: 0,
        };
    };
    let runtime_guard = RUNTIME.lock();
    let sectors = runtime_guard
        .as_ref()
        .and_then(|runtime| {
            runtime
                .storage_devices
                .iter()
                .find(|storage| storage.index == index)
        })
        .map(|storage| storage.sectors)
        .unwrap_or(0);
    crate::storage::BlockDeviceInfo {
        device,
        present: sectors > 0,
        sectors,
    }
}

pub fn storage_read_sector(
    device: crate::storage::BlockDevice,
    lba: u32,
    buf: &mut [u8; 512],
) -> bool {
    storage_transfer_sector(device, lba, buf, false)
}

pub fn storage_write_sector(
    device: crate::storage::BlockDevice,
    lba: u32,
    buf: &[u8; 512],
) -> bool {
    let mut temp = [0u8; 512];
    temp.copy_from_slice(buf);
    storage_transfer_sector(device, lba, &mut temp, true)
}

pub fn storage_flush(device: crate::storage::BlockDevice) -> bool {
    let Some(index) = device.usb_index() else {
        return false;
    };
    let mut runtime_guard = RUNTIME.lock();
    let Some(runtime) = runtime_guard.as_mut() else {
        return false;
    };
    let ActiveState {
        info,
        event_ring,
        storage_devices,
        ..
    } = runtime;
    let Some(storage) = storage_devices
        .iter_mut()
        .find(|storage| storage.index == index)
    else {
        return false;
    };
    scsi_flush(info, event_ring, storage).is_ok()
}

fn storage_transfer_sector(
    device: crate::storage::BlockDevice,
    lba: u32,
    buf: &mut [u8; 512],
    write: bool,
) -> bool {
    let Some(index) = device.usb_index() else {
        return false;
    };
    let mut runtime_guard = RUNTIME.lock();
    let Some(runtime) = runtime_guard.as_mut() else {
        return false;
    };
    let ActiveState {
        info,
        event_ring,
        storage_devices,
        ..
    } = runtime;
    let Some(storage) = storage_devices
        .iter_mut()
        .find(|storage| storage.index == index)
    else {
        return false;
    };
    if write {
        scsi_write_sector(info, event_ring, storage, lba, buf).is_ok()
    } else {
        scsi_read_sector(info, event_ring, storage, lba, buf).is_ok()
    }
}
