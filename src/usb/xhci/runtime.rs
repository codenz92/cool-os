fn handle_runtime_transfer_event(runtime: &mut ActiveState, event: EventTrb) {
    let completion = TransferCompletion {
        ptr: event.parameter & !0xFu64,
        completion_code: event.completion_code(),
        slot_id: event.slot_id(),
        endpoint_id: event.endpoint_id(),
        residual: event.residual(),
    };

    let Some(device) = runtime.devices.iter_mut().find(|device| {
        device.slot_id == completion.slot_id && device.endpoint_dci == completion.endpoint_id
    }) else {
        if runtime.storage_devices.iter().any(|device| {
            device.slot_id == completion.slot_id
                && (device.bulk_in.dci == completion.endpoint_id
                    || device.bulk_out.dci == completion.endpoint_id)
        }) {
            runtime.last_runtime_note = format!(
                "deferred storage event slot {} ep {}",
                completion.slot_id, completion.endpoint_id,
            );
            return;
        }
        println!(
            "[xhci] runtime transfer for unknown device slot={} ep={} ptr={:#x} code={}",
            completion.slot_id, completion.endpoint_id, completion.ptr, completion.completion_code,
        );
        return;
    };

    if completion.ptr != device.report_trb_phys {
        device.error_count = device.error_count.saturating_add(1);
        device.last_completion_code = completion.completion_code;
        runtime.last_runtime_note = format!(
            "slot {} ep {} ptr mismatch",
            completion.slot_id, completion.endpoint_id,
        );
        println!(
            "[xhci] runtime transfer ptr mismatch slot={} ep={} got={:#x} expected={:#x}",
            completion.slot_id, completion.endpoint_id, completion.ptr, device.report_trb_phys,
        );
        return;
    }

    device.last_completion_code = completion.completion_code;
    if !completion_is_success_like(completion.completion_code) {
        device.error_count = device.error_count.saturating_add(1);
        runtime.last_runtime_note = format!(
            "slot {} ep {} code {}",
            completion.slot_id, completion.endpoint_id, completion.completion_code,
        );
        println!(
            "[xhci] HID transfer failed slot={} ep={} code={}",
            completion.slot_id, completion.endpoint_id, completion.completion_code,
        );
        return;
    }

    let actual_len = device
        .report_request_len
        .saturating_sub(completion.residual as usize);
    device.report_count = device.report_count.saturating_add(1);
    device.last_report_len = actual_len;
    runtime.last_runtime_note = format!(
        "slot {} {} {}B",
        device.slot_id,
        hid_kind_name(device.kind),
        actual_len,
    );
    dispatch_hid_report(device, actual_len);

    if let Err(err) = queue_interrupt_transfer_by_base(runtime.db_base, device) {
        device.error_count = device.error_count.saturating_add(1);
        runtime.last_runtime_note = format!(
            "slot {} ep {} requeue {}",
            device.slot_id, device.endpoint_dci, err,
        );
        println!(
            "[xhci] failed to requeue HID transfer slot={} ep={} err={}",
            device.slot_id, device.endpoint_dci, err
        );
    }
}

fn handle_runtime_port_status_change(runtime: &mut ActiveState, port_num: u8) {
    let Some(proto) = protocol_for_port(&runtime.info.protocols, port_num).cloned() else {
        let portsc = read_portsc(&runtime.info, port_num);
        clear_port_changes(&runtime.info, port_num);
        runtime.last_runtime_note = format!(
            "port {} change ccs={} ped={} speed_id={}",
            port_num,
            (portsc & PORTSC_CCS != 0) as u8,
            (portsc & PORTSC_PED != 0) as u8,
            port_speed_id(portsc),
        );
        println!(
            "[xhci] runtime event: port {} status change portsc={:#x}",
            port_num, portsc,
        );
        return;
    };

    let initial_portsc = read_portsc(&runtime.info, port_num);
    println!(
        "[xhci] runtime event: port {} status change portsc={:#x}",
        port_num, initial_portsc,
    );

    let had_connection_change = initial_portsc & PORTSC_CSC != 0;
    let had_existing_device = runtime.devices.iter().any(|device| device.port_num == port_num)
        || runtime
            .storage_devices
            .iter()
            .any(|device| device.port_num == port_num);

    clear_port_changes(&runtime.info, port_num);
    let current_portsc = read_portsc(&runtime.info, port_num);

    if current_portsc & PORTSC_CCS == 0 {
        remove_port_devices(runtime, port_num);
        replace_port_status_lines(
            runtime,
            port_num,
            vec![format!(
                "USB: port {} {} disconnected",
                port_num, proto.label
            )],
        );
        runtime.last_runtime_note = format!("port {} disconnected", port_num);
        return;
    }

    if had_existing_device && (had_connection_change || current_portsc & PORTSC_PED == 0) {
        remove_port_devices(runtime, port_num);
    }

    if runtime.devices.iter().any(|device| device.port_num == port_num)
        || runtime
            .storage_devices
            .iter()
            .any(|device| device.port_num == port_num)
    {
        runtime.last_runtime_note =
            format!("port {} {} status change handled", port_num, proto.label);
        return;
    }

    let portsc =
        match prepare_port_for_probe(&runtime.info, &mut runtime.event_ring, port_num, &proto) {
            Ok(Some(portsc)) => portsc,
            Ok(None) => {
                replace_port_status_lines(
                    runtime,
                    port_num,
                    vec![format!(
                        "USB: port {} {} disconnected",
                        port_num, proto.label
                    )],
                );
                runtime.last_runtime_note = format!("port {} disconnected", port_num);
                return;
            }
            Err(err) => {
                replace_port_status_lines(
                    runtime,
                    port_num,
                    vec![format!(
                        "USB: port {} {} reset failed: {}",
                        port_num, proto.label, err
                    )],
                );
                runtime.last_runtime_note = format!("port {} reset failed {}", port_num, err);
                return;
            }
        };

    if portsc & PORTSC_PED == 0 {
        replace_port_status_lines(
            runtime,
            port_num,
            vec![format!(
                "USB: port {} {} connected but not enabled",
                port_num, proto.label
            )],
        );
        runtime.last_runtime_note = format!("port {} connected but not enabled", port_num);
        return;
    }

    match prime_default_control_endpoint(
        &runtime.info,
        runtime.dcbaa_virt,
        &mut runtime.cmd_ring,
        &mut runtime.event_ring,
        &proto,
        port_num,
        portsc,
    ) {
        Ok((lines, mut devices, mut storage_devices)) => {
            replace_port_status_lines(runtime, port_num, lines);
            runtime.devices.append(&mut devices);
            runtime.storage_devices.append(&mut storage_devices);
            runtime.last_runtime_note = format!("port {} {} attached", port_num, proto.label);
        }
        Err(err) => {
            replace_port_status_lines(
                runtime,
                port_num,
                vec![format!(
                    "USB: port {} {} prime failed: {}",
                    port_num, proto.label, err
                )],
            );
            runtime.last_runtime_note = format!("port {} prime failed {}", port_num, err);
        }
    }
}

fn remove_port_devices(runtime: &mut ActiveState, port_num: u8) {
    let mut slot_ids = Vec::new();
    for device in runtime.devices.iter() {
        if device.port_num != port_num || slot_ids.contains(&device.slot_id) {
            continue;
        }
        slot_ids.push(device.slot_id);
    }
    for device in runtime.storage_devices.iter() {
        if device.port_num != port_num || slot_ids.contains(&device.slot_id) {
            continue;
        }
        slot_ids.push(device.slot_id);
    }

    for slot_id in slot_ids {
        let _ = disable_slot(
            &runtime.info,
            &mut runtime.cmd_ring,
            &mut runtime.event_ring,
            slot_id,
        );
    }

    runtime.devices.retain(|device| device.port_num != port_num);
    runtime
        .storage_devices
        .retain(|device| device.port_num != port_num);
    crate::device_registry::disconnect_usb_storage_port(port_num);
}

fn replace_port_status_lines(runtime: &mut ActiveState, port_num: u8, mut new_lines: Vec<String>) {
    let prefix = format!("USB: port {} ", port_num);
    runtime
        .port_status
        .retain(|line| !line.starts_with(&prefix));
    runtime.port_status.append(&mut new_lines);
}

fn dispatch_hid_report(device: &mut HidDeviceState, actual_len: usize) {
    let mut report = [0u8; BOOT_KEYBOARD_REPORT_BYTES];
    let len = actual_len.min(device.report_request_len).min(report.len());
    for (idx, byte) in report.iter_mut().take(len).enumerate() {
        *byte = unsafe {
            core::ptr::read_volatile((device.report_buffer_virt + idx as u64) as *const u8)
        };
    }

    match device.kind {
        HID_KIND_KEYBOARD if len >= BOOT_KEYBOARD_REPORT_BYTES => {
            crate::keyboard::handle_usb_boot_report(&report);
        }
        HID_KIND_MOUSE if len >= 3 => {
            crate::mouse::handle_usb_boot_report(&report[..len]);
        }
        HID_KIND_TABLET if len >= TABLET_REPORT_BYTES => {
            crate::mouse::handle_usb_tablet_report(&report[..len]);
        }
        _ => {}
    }
}

fn queue_interrupt_transfer_by_base(
    db_base: u64,
    device: &mut HidDeviceState,
) -> Result<(), &'static str> {
    if device.report_request_len == 0 {
        return Err("interrupt report length is zero");
    }

    device.report_trb_phys = push_transfer_trb(
        &mut device.report_ring,
        device.report_buffer_phys,
        device.report_request_len as u32,
        (TRB_TYPE_NORMAL << 10) | TRB_IOC | TRB_DIR_IN,
    );
    fence(Ordering::SeqCst);
    unsafe {
        write_u32(
            db_base + device.slot_id as u64 * 4,
            device.endpoint_dci as u32,
        );
    }
    Ok(())
}

fn completion_is_success_like(code: u8) -> bool {
    code == COMPLETION_SUCCESS || code == COMPLETION_SHORT_PACKET
}
