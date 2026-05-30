fn read_portsc(info: &XhciInfo, port_num: u8) -> u32 {
    read_portsc_by_op_base(info.op_base, port_num)
}

fn portsc_addr(info: &XhciInfo, port_num: u8) -> u64 {
    portsc_addr_by_op_base(info.op_base, port_num)
}

fn portsc_addr_by_op_base(op_base: u64, port_num: u8) -> u64 {
    op_base + OP_PORTSC_BASE + 0x10 * (port_num as u64 - 1)
}

fn read_portsc_by_op_base(op_base: u64, port_num: u8) -> u32 {
    unsafe { read_u32(portsc_addr_by_op_base(op_base, port_num)) }
}

fn port_speed_id(portsc: u32) -> u8 {
    ((portsc & PORTSC_SPEED_MASK) >> PORTSC_SPEED_SHIFT) as u8
}

fn clear_port_changes(info: &XhciInfo, port_num: u8) {
    clear_port_changes_by_op_base(info.op_base, port_num);
}

fn clear_port_changes_by_op_base(op_base: u64, port_num: u8) {
    let portsc = read_portsc_by_op_base(op_base, port_num);
    let change_bits = portsc & PORTSC_CHANGE_BITS;
    if change_bits == 0 {
        return;
    }

    unsafe {
        write_u32(
            portsc_addr_by_op_base(op_base, port_num),
            (portsc & PORTSC_PP) | change_bits,
        );
    }
}

fn reset_port(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    port_num: u8,
) -> Result<u32, &'static str> {
    let portsc = read_portsc(info, port_num);
    unsafe {
        write_u32(
            portsc_addr(info, port_num),
            (portsc & PORTSC_PP) | (portsc & PORTSC_CHANGE_BITS) | PORTSC_PR,
        );
    }

    let _ = wait_for_port_status_change(info, event_ring, port_num)?;
    clear_port_changes(info, port_num);
    if read_portsc(info, port_num) & PORTSC_PR != 0 {
        wait_until("port reset clear", || {
            read_portsc(info, port_num) & PORTSC_PR == 0
        })?;
    }

    Ok(read_portsc(info, port_num))
}

fn wait_for_port_status_change(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    port_num: u8,
) -> Result<u32, &'static str> {
    for _ in 0..SPIN_TIMEOUT {
        if let Some(event) = next_event(info, event_ring) {
            match event.trb_type() as u32 {
                TRB_TYPE_PORT_STATUS_CHANGE => {
                    if event.port_id() == port_num {
                        return Ok(read_portsc(info, port_num));
                    }
                    println!(
                        "[xhci] ignoring port change for port {} while waiting on port {}",
                        event.port_id(),
                        port_num,
                    );
                }
                TRB_TYPE_CMD_COMPLETION => {
                    println!(
                        "[xhci] unexpected command completion while waiting for port {} ptr={:#x} code={} slot={}",
                        port_num,
                        event.parameter & !0xFu64,
                        event.completion_code(),
                        event.slot_id(),
                    );
                }
                _ => {
                    println!(
                        "[xhci] unexpected event while waiting for port {} type={} code={} param={:#x}",
                        port_num,
                        event.trb_type(),
                        event.completion_code(),
                        event.parameter,
                    );
                }
            }
        }
        core::hint::spin_loop();
    }

    println!(
        "[xhci] timeout while waiting for port {} status change",
        port_num
    );
    Err("port change timeout")
}

fn wait_for_transfer_completion(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    slot_id: u8,
    endpoint_id: u8,
    expected_trb_phys: u64,
) -> Result<TransferCompletion, &'static str> {
    for _ in 0..SPIN_TIMEOUT {
        if let Some(event) = next_event(info, event_ring) {
            match event.trb_type() as u32 {
                TRB_TYPE_TRANSFER_EVENT => {
                    let completion = TransferCompletion {
                        ptr: event.parameter & !0xFu64,
                        completion_code: event.completion_code(),
                        slot_id: event.slot_id(),
                        endpoint_id: event.endpoint_id(),
                        residual: event.residual(),
                    };

                    if completion.slot_id == slot_id && completion.endpoint_id == endpoint_id {
                        if completion.ptr == expected_trb_phys
                            || !completion_is_success_like(completion.completion_code)
                        {
                            return Ok(completion);
                        }
                        println!(
                            "[xhci] transfer event slot={} ep={} ptr={:#x} code={} residual={}",
                            completion.slot_id,
                            completion.endpoint_id,
                            completion.ptr,
                            completion.completion_code,
                            completion.residual,
                        );
                    }
                }
                TRB_TYPE_PORT_STATUS_CHANGE => {
                    println!(
                        "[xhci] event: port {} changed during transfer",
                        event.port_id()
                    );
                }
                TRB_TYPE_CMD_COMPLETION => {
                    println!(
                        "[xhci] unexpected command completion during transfer ptr={:#x} code={} slot={}",
                        event.parameter & !0xFu64,
                        event.completion_code(),
                        event.slot_id(),
                    );
                }
                _ => {
                    println!(
                        "[xhci] unexpected event during transfer type={} code={} param={:#x} status={:#x}",
                        event.trb_type(),
                        event.completion_code(),
                        event.parameter,
                        event.status,
                    );
                }
            }
        }
        core::hint::spin_loop();
    }

    println!(
        "[xhci] timeout while waiting for transfer completion slot={} ep={} ptr={:#x}",
        slot_id, endpoint_id, expected_trb_phys,
    );
    Err("transfer completion timeout")
}

fn ring_host_doorbell(info: &XhciInfo) {
    fence(Ordering::SeqCst);
    unsafe {
        write_u32(info.db_base, 0);
    }
}

fn ring_device_doorbell(info: &XhciInfo, slot_id: u8, dci: u8) {
    fence(Ordering::SeqCst);
    unsafe {
        write_u32(info.db_base + slot_id as u64 * 4, dci as u32);
    }
}

fn push_command_trb(ring: &mut CommandRingState, parameter: u64, status: u32, control: u32) -> u64 {
    let trb_phys = ring.phys + ring.enqueue_idx as u64 * 16;
    let trb_virt = ring.virt + ring.enqueue_idx as u64 * 16;
    let (next_enqueue_idx, next_cycle, wraps) =
        next_ring_enqueue_state(ring.enqueue_idx, ring.cycle, COMMAND_RING_TRBS);
    let control = control | if ring.cycle { TRB_CYCLE } else { 0 };

    unsafe {
        write_u64(trb_virt, parameter);
        write_u32(trb_virt + 8, status);
        if wraps {
            write_link_trb_cycle(ring.virt, COMMAND_RING_TRBS, ring.cycle);
        }
        write_u32(trb_virt + 12, control);
    }

    ring.enqueue_idx = next_enqueue_idx;
    ring.cycle = next_cycle;

    trb_phys
}

fn push_transfer_trb(
    ring: &mut TransferRingState,
    parameter: u64,
    status: u32,
    control: u32,
) -> u64 {
    let trb_phys = ring.phys + ring.enqueue_idx as u64 * 16;
    let trb_virt = ring.virt + ring.enqueue_idx as u64 * 16;
    let (next_enqueue_idx, next_cycle, wraps) =
        next_ring_enqueue_state(ring.enqueue_idx, ring.cycle, ring.size);
    let control = control | if ring.cycle { TRB_CYCLE } else { 0 };

    unsafe {
        write_u64(trb_virt, parameter);
        write_u32(trb_virt + 8, status);
        if wraps {
            write_link_trb_cycle(ring.virt, ring.size, ring.cycle);
        }
        write_u32(trb_virt + 12, control);
    }

    ring.enqueue_idx = next_enqueue_idx;
    ring.cycle = next_cycle;

    trb_phys
}

fn next_ring_enqueue_state(
    enqueue_idx: usize,
    cycle: bool,
    ring_size: usize,
) -> (usize, bool, bool) {
    let next_idx = enqueue_idx + 1;
    if next_idx == ring_size - 1 {
        (0, !cycle, true)
    } else {
        (next_idx, cycle, false)
    }
}

pub fn transfer_ring_cycle_refresh_for_test() -> bool {
    let mut enqueue_idx = 0usize;
    let mut cycle = true;
    let mut wraps = 0usize;
    let mut observed = [false; 4];

    for _ in 0..12 {
        let (next_idx, next_cycle, wrapped) = next_ring_enqueue_state(enqueue_idx, cycle, 5);
        if wrapped {
            if wraps >= observed.len() {
                return false;
            }
            observed[wraps] = cycle;
            wraps += 1;
        }
        enqueue_idx = next_idx;
        cycle = next_cycle;
    }

    wraps == 3 && observed[0] && !observed[1] && observed[2]
}

fn wait_for_command_completion(
    info: &XhciInfo,
    event_ring: &mut EventRingState,
    expected_trb_phys: u64,
) -> Result<CommandCompletion, &'static str> {
    for _ in 0..SPIN_TIMEOUT {
        if let Some(event) = next_event(info, event_ring) {
            match event.trb_type() as u32 {
                TRB_TYPE_CMD_COMPLETION => {
                    let completion = CommandCompletion {
                        ptr: event.parameter & !0xFu64,
                        completion_code: event.completion_code(),
                        slot_id: event.slot_id(),
                    };
                    if completion.ptr == expected_trb_phys {
                        return Ok(completion);
                    }
                    println!(
                        "[xhci] unexpected command completion ptr={:#x} code={} slot={}",
                        completion.ptr, completion.completion_code, completion.slot_id,
                    );
                }
                TRB_TYPE_PORT_STATUS_CHANGE => {
                    println!(
                        "[xhci] event: port status change param={:#x} status={:#x} control={:#x}",
                        event.parameter, event.status, event.control,
                    );
                }
                _ => {
                    println!(
                        "[xhci] event: type={} code={} param={:#x} status={:#x} control={:#x}",
                        event.trb_type(),
                        event.completion_code(),
                        event.parameter,
                        event.status,
                        event.control,
                    );
                }
            }
        }
        core::hint::spin_loop();
    }

    println!(
        "[xhci] timeout while waiting for command completion ptr={:#x}",
        expected_trb_phys,
    );
    Err("command completion timeout")
}

fn next_event(info: &XhciInfo, ring: &mut EventRingState) -> Option<EventTrb> {
    next_event_by_base(info.rt_base, ring)
}

fn next_event_by_base(rt_base: u64, ring: &mut EventRingState) -> Option<EventTrb> {
    let trb_virt = ring.virt + ring.dequeue_idx as u64 * 16;
    let control = unsafe { read_u32(trb_virt + 12) };
    if (control & TRB_CYCLE != 0) != ring.cycle {
        return None;
    }

    let event = EventTrb {
        parameter: unsafe { read_u64(trb_virt) },
        status: unsafe { read_u32(trb_virt + 8) },
        control,
    };
    advance_event_ring_by_base(rt_base, ring);
    Some(event)
}

fn advance_event_ring_by_base(rt_base: u64, ring: &mut EventRingState) {
    ring.dequeue_idx += 1;
    if ring.dequeue_idx == EVENT_RING_TRBS {
        ring.dequeue_idx = 0;
        ring.cycle = !ring.cycle;
    }

    let erdp = ring.phys + ring.dequeue_idx as u64 * 16;
    unsafe {
        write_u64(rt_base + RT_IR0 + IR0_ERDP, erdp | ERDP_EHB_CLEAR);
    }
}
