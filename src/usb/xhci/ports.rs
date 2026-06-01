fn run_command_ring_noop(
    info: &XhciInfo,
    cmd_ring: &mut CommandRingState,
    event_ring: &mut EventRingState,
) -> Result<(), &'static str> {
    let trb_phys = push_command_trb(cmd_ring, 0, 0, TRB_TYPE_NOOP_CMD << 10);
    fence(Ordering::SeqCst);
    unsafe {
        write_u32(info.db_base, 0);
    }

    let completion = wait_for_command_completion(info, event_ring, trb_phys)?;
    if completion.completion_code != COMPLETION_SUCCESS {
        println!(
            "[xhci] command ring no-op failed code={} ptr={:#x} slot={}",
            completion.completion_code, completion.ptr, completion.slot_id,
        );
        return Err("command ring no-op failed");
    }

    println!(
        "[xhci] command ring no-op complete ptr={:#x} slot={}",
        completion.ptr, completion.slot_id,
    );
    Ok(())
}

fn prepare_port_for_probe(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    port_num: u8,
    proto: &SupportedProtocol,
) -> Result<Option<u32>, &'static str> {
    let mut portsc = read_portsc(info, port_num);
    if portsc & PORTSC_CCS == 0 {
        clear_port_changes(info, port_num);
        return Ok(None);
    }

    clear_port_changes(info, port_num);
    portsc = read_portsc(info, port_num);

    if proto.major == 2 && portsc & PORTSC_PED == 0 {
        portsc = reset_port(info, event_ring, port_num)?;
    } else if proto.major >= 3 && portsc & PORTSC_PED == 0 {
        let _ = wait_until("usb3 port enable", || {
            let current = read_portsc(info, port_num);
            current & PORTSC_PED != 0 || current & PORTSC_CCS == 0
        });
        clear_port_changes(info, port_num);
        portsc = read_portsc(info, port_num);
    }

    if portsc & PORTSC_CCS == 0 {
        return Ok(None);
    }

    Ok(Some(portsc))
}

fn prime_attached_ports(
    info: &XhciInfo,
    dcbaa_virt: u64,
    cmd_ring: &mut CommandRingState,
    event_ring: &mut EventRingState,
) -> (Vec<String>, Vec<HidDeviceState>, Vec<UsbStorageDeviceState>) {
    let mut status = Vec::new();
    let mut devices = Vec::new();
    let mut storage_devices = Vec::new();

    for port_num in 1..=info.max_ports {
        let Some(proto) = protocol_for_port(&info.protocols, port_num) else {
            continue;
        };

        let initial_portsc = read_portsc(info, port_num);
        if initial_portsc & PORTSC_CCS == 0 {
            continue;
        }

        let portsc = match prepare_port_for_probe(info, event_ring, port_num, proto) {
            Ok(Some(portsc)) => portsc,
            Ok(None) => {
                status.push(format!(
                    "USB: port {} {} disconnected during probe",
                    port_num, proto.label
                ));
                continue;
            }
            Err(err) => {
                status.push(format!(
                    "USB: port {} {} reset failed: {}",
                    port_num, proto.label, err
                ));
                continue;
            }
        };

        if portsc & PORTSC_PED == 0 {
            status.push(format!(
                "USB: port {} {} connected but not enabled",
                port_num, proto.label
            ));
            continue;
        }

        match prime_default_control_endpoint(
            info, dcbaa_virt, cmd_ring, event_ring, proto, port_num, portsc,
        ) {
            Ok((lines, maybe_devices, maybe_storage_devices)) => {
                status.extend(lines);
                devices.extend(maybe_devices);
                storage_devices.extend(maybe_storage_devices);
            }
            Err(err) => status.push(format!(
                "USB: port {} {} prime failed: {}",
                port_num, proto.label, err
            )),
        }
    }

    (status, devices, storage_devices)
}

fn prime_default_control_endpoint(
    info: &XhciInfo,
    dcbaa_virt: u64,
    cmd_ring: &mut CommandRingState,
    event_ring: &mut EventRingState,
    proto: &SupportedProtocol,
    port_num: u8,
    portsc: u32,
) -> Result<(Vec<String>, Vec<HidDeviceState>, Vec<UsbStorageDeviceState>), &'static str> {
    let speed_id = port_speed_id(portsc);
    if speed_id == 0 {
        return Err("port speed undefined");
    }

    let slot_id = enable_slot(info, cmd_ring, event_ring, proto.slot_type)?;
    let mut device =
        match build_default_control_device(info, dcbaa_virt, slot_id, port_num, speed_id) {
            Ok(device) => device,
            Err(err) => {
                let _ = disable_slot(info, cmd_ring, event_ring, slot_id);
                return Err(err);
            }
        };

    if let Err(err) = address_device(
        info,
        cmd_ring,
        event_ring,
        slot_id,
        device.input_ctx_phys,
        true,
    ) {
        let _ = disable_slot(info, cmd_ring, event_ring, slot_id);
        return Err(err);
    }

    let descriptor8 = match read_device_descriptor_header(
        info,
        event_ring,
        slot_id,
        &mut device.transfer_ring,
        device.descriptor_phys,
        device.descriptor_virt,
    ) {
        Ok(bytes) => bytes,
        Err(err) => {
            let _ = disable_slot(info, cmd_ring, event_ring, slot_id);
            return Err(err);
        }
    };

    let descriptor = match read_device_descriptor(
        info,
        event_ring,
        slot_id,
        &mut device.transfer_ring,
        device.descriptor_phys,
        device.descriptor_virt,
    ) {
        Ok(descriptor) => descriptor,
        Err(err) => {
            let _ = disable_slot(info, cmd_ring, event_ring, slot_id);
            return Err(err);
        }
    };

    update_ep0_max_packet_size(info, &device, descriptor.max_packet_size0);
    if let Err(err) = address_device(
        info,
        cmd_ring,
        event_ring,
        slot_id,
        device.input_ctx_phys,
        false,
    ) {
        let _ = disable_slot(info, cmd_ring, event_ring, slot_id);
        return Err(err);
    }

    let config = match read_configuration_descriptor(
        info,
        event_ring,
        slot_id,
        &mut device.transfer_ring,
        device.descriptor_phys,
        device.descriptor_virt,
    ) {
        Ok(config) => config,
        Err(err) => {
            let _ = disable_slot(info, cmd_ring, event_ring, slot_id);
            return Err(err);
        }
    };

    let hid_interfaces = parse_hid_interfaces(&config);
    let msc_interfaces = parse_msc_interfaces(&config);
    let mut status = Vec::new();
    let mut devices = Vec::new();
    let mut storage_devices = Vec::new();
    let config_value = config
        .get(5)
        .copied()
        .ok_or("configuration value missing")?;

    println!(
        "[xhci] slot {} device vid={:04x} pid={:04x} bcdUSB={:04x} dev={:04x} class={:02x}/{:02x}/{:02x} configs={}",
        slot_id,
        descriptor.vendor_id,
        descriptor.product_id,
        descriptor.usb_bcd,
        descriptor.device_bcd,
        descriptor.class,
        descriptor.subclass,
        descriptor.protocol,
        descriptor.configurations,
    );

    status.push(format!(
        "USB: port {} {} slot={} speed={} vid={:04x} pid={:04x} bcdUSB={:04x} dev={:04x} ep0_mps={} mps0_raw={} default_mps={}",
        port_num,
        proto.label,
        slot_id,
        port_speed_name(proto, speed_id),
        descriptor.vendor_id,
        descriptor.product_id,
        descriptor.usb_bcd,
        descriptor.device_bcd,
        descriptor.max_packet_size0,
        descriptor8[7],
        device.default_mps,
    ));

    if hid_interfaces.is_empty() && msc_interfaces.is_empty() {
        println!(
            "[xhci] slot {} config0 parsed but no boot HID or mass-storage interfaces were found",
            slot_id
        );
        status.push(format!(
            "USB: port {} slot={} no boot HID or mass-storage interfaces in config 0",
            port_num, slot_id,
        ));
        return Ok((status, devices, storage_devices));
    }

    if let Err(err) = set_configuration(
        info,
        event_ring,
        slot_id,
        &mut device.transfer_ring,
        config_value,
    ) {
        let _ = disable_slot(info, cmd_ring, event_ring, slot_id);
        return Err(err);
    }

    for msc in msc_interfaces {
        let configured = match activate_msc_interface(
            info,
            cmd_ring,
            event_ring,
            slot_id,
            port_num,
            &msc,
            &mut device,
        ) {
            Ok(configured) => configured,
            Err(err) => {
                let _ = disable_slot(info, cmd_ring, event_ring, slot_id);
                return Err(err);
            }
        };
        println!(
            "[xhci] slot {} MSC usb{} iface={} alt={} in={:#04x} out={:#04x} sectors={} block={}",
            slot_id,
            configured.index,
            configured.interface_number,
            msc.alternate_setting,
            configured.bulk_in.address,
            configured.bulk_out.address,
            configured.sectors,
            configured.block_size,
        );
        status.push(format!(
            "USB: port {} slot={} MSC usb{} iface={} alt={} in={:#04x} out={:#04x} sectors={} block={}",
            port_num,
            slot_id,
            configured.index,
            configured.interface_number,
            msc.alternate_setting,
            configured.bulk_in.address,
            configured.bulk_out.address,
            configured.sectors,
            configured.block_size,
        ));
        crate::device_registry::set_usb_storage(
            configured.index,
            port_num,
            configured.sectors,
            configured.block_size,
        );
        storage_devices.push(configured);
    }

    for hid in hid_interfaces {
        let configured = match activate_hid_interface(
            info,
            cmd_ring,
            event_ring,
            slot_id,
            port_num,
            speed_id,
            &hid,
            &mut device,
        ) {
            Ok(configured) => configured,
            Err(err) => {
                let _ = disable_slot(info, cmd_ring, event_ring, slot_id);
                return Err(err);
            }
        };
        println!(
            "[xhci] slot {} hid {} iface={} alt={} ep={:#04x} mps={} interval={} report_desc={}",
            slot_id,
            hid_kind_name(configured.kind),
            configured.interface_number,
            hid.alternate_setting,
            configured.endpoint_address,
            hid.max_packet_size,
            configured.interval,
            hid.report_descriptor_len,
        );
        status.push(format!(
            "USB: port {} slot={} HID {} iface={} alt={} ep={:#04x} mps={} interval={} report_desc={}",
            port_num,
            slot_id,
            hid_kind_name(configured.kind),
            configured.interface_number,
            hid.alternate_setting,
            configured.endpoint_address,
            hid.max_packet_size,
            configured.interval,
            hid.report_descriptor_len,
        ));
        devices.push(configured);
    }

    for storage in storage_devices.iter_mut() {
        storage.control_ring = device.transfer_ring;
    }

    Ok((status, devices, storage_devices))
}
