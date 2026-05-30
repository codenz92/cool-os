fn user_path(path_ptr: *const u8, path_len: u64) -> Option<&'static str> {
    let bytes = user_slice(path_ptr, path_len, MAX_USER_STRING)?;
    core::str::from_utf8(bytes).ok()
}

fn user_descriptor4(desc_ptr: *const u8) -> Option<[u64; 4]> {
    let bytes = user_slice(desc_ptr, 32, 32)?;
    Some([
        u64::from_le_bytes(bytes[0..8].try_into().ok()?),
        u64::from_le_bytes(bytes[8..16].try_into().ok()?),
        u64::from_le_bytes(bytes[16..24].try_into().ok()?),
        u64::from_le_bytes(bytes[24..32].try_into().ok()?),
    ])
}

fn user_descriptor5(desc_ptr: *const u8) -> Option<[u64; 5]> {
    let bytes = user_slice(desc_ptr, 40, 40)?;
    Some([
        u64::from_le_bytes(bytes[0..8].try_into().ok()?),
        u64::from_le_bytes(bytes[8..16].try_into().ok()?),
        u64::from_le_bytes(bytes[16..24].try_into().ok()?),
        u64::from_le_bytes(bytes[24..32].try_into().ok()?),
        u64::from_le_bytes(bytes[32..40].try_into().ok()?),
    ])
}

fn user_descriptor6(desc_ptr: *const u8) -> Option<[u64; 6]> {
    let bytes = user_slice(desc_ptr, 48, 48)?;
    Some([
        u64::from_le_bytes(bytes[0..8].try_into().ok()?),
        u64::from_le_bytes(bytes[8..16].try_into().ok()?),
        u64::from_le_bytes(bytes[16..24].try_into().ok()?),
        u64::from_le_bytes(bytes[24..32].try_into().ok()?),
        u64::from_le_bytes(bytes[32..40].try_into().ok()?),
        u64::from_le_bytes(bytes[40..48].try_into().ok()?),
    ])
}

fn resolve_task_path(path: &str) -> String {
    if path.starts_with('/') {
        return crate::vfs::normalize_path(path);
    }
    let cwd = crate::scheduler::current_cwd();
    if cwd == "/" {
        crate::vfs::normalize_path(&alloc::format!("/{}", path))
    } else {
        crate::vfs::normalize_path(&alloc::format!("{}/{}", cwd, path))
    }
}

fn parse_user_arg_strings(arg_pairs_ptr: u64, argc: u64, max_argc: u64) -> Option<Vec<String>> {
    if argc > max_argc {
        return None;
    }
    let arg_pairs = if argc == 0 {
        &[][..]
    } else {
        let pair_bytes = argc.saturating_mul(16);
        user_slice(arg_pairs_ptr as *const u8, pair_bytes, max_argc * 16)?
    };

    let mut arg_strings = Vec::new();
    if arg_strings.try_reserve_exact(argc as usize).is_err() {
        return None;
    }
    for idx in 0..argc as usize {
        let base = idx * 16;
        let ptr = u64::from_le_bytes(arg_pairs[base..base + 8].try_into().ok()?);
        let len = u64::from_le_bytes(arg_pairs[base + 8..base + 16].try_into().ok()?);
        let bytes = user_slice(ptr as *const u8, len, MAX_USER_STRING)?;
        let arg = core::str::from_utf8(bytes).ok()?;
        arg_strings.push(String::from(arg));
    }
    Some(arg_strings)
}

fn parse_user_fd_mappings(
    fd_pairs_ptr: u64,
    fd_count: u64,
    max_fd_count: u64,
) -> Option<Vec<(usize, usize)>> {
    if fd_count > max_fd_count {
        return None;
    }
    let pairs = if fd_count == 0 {
        &[][..]
    } else {
        let pair_bytes = fd_count.saturating_mul(16);
        user_slice(fd_pairs_ptr as *const u8, pair_bytes, max_fd_count * 16)?
    };
    let mut out = Vec::new();
    if out.try_reserve_exact(fd_count as usize).is_err() {
        return None;
    }
    for idx in 0..fd_count as usize {
        let base = idx * 16;
        let parent_fd = u64::from_le_bytes(pairs[base..base + 8].try_into().ok()?);
        let child_fd = u64::from_le_bytes(pairs[base + 8..base + 16].try_into().ok()?);
        if parent_fd > usize::MAX as u64 || child_fd > usize::MAX as u64 {
            return None;
        }
        out.push((parent_fd as usize, child_fd as usize));
    }
    Some(out)
}

fn write_record_u64(out: &mut [u8], offset: usize, value: u64) {
    out[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn append_dir_listing_byte(out: &mut [u8], written: &mut usize, byte: u8) -> bool {
    if *written >= out.len() {
        return false;
    }
    out[*written] = byte;
    *written += 1;
    true
}

fn append_dir_listing_bytes(out: &mut [u8], written: &mut usize, bytes: &[u8]) -> bool {
    for &byte in bytes {
        if !append_dir_listing_byte(out, written, byte) {
            return false;
        }
    }
    true
}

fn append_dir_listing_u64(out: &mut [u8], written: &mut usize, mut value: u64) -> bool {
    let mut digits = [0u8; 20];
    let mut len = 0usize;
    if value == 0 {
        return append_dir_listing_byte(out, written, b'0');
    }
    while value > 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    while len > 0 {
        len -= 1;
        if !append_dir_listing_byte(out, written, digits[len]) {
            return false;
        }
    }
    true
}

fn sys_yield() {
    // No-op: the preemptive timer will preempt voluntarily yielding tasks.
}

fn valid_user_address_range(ptr: u64, len: u64, max_len: u64) -> bool {
    if ptr == 0 || len == 0 || len > max_len {
        return false;
    }
    let Some(end) = ptr.checked_add(len) else {
        return false;
    };
    end <= crate::vmm::USER_TOP && ptr < crate::vmm::USER_TOP
}

fn validate_user_range(ptr: u64, len: u64, max_len: u64, writable: bool) -> bool {
    valid_user_address_range(ptr, len, max_len)
        && crate::vmm::user_range_accessible(ptr, len, writable)
}

pub fn validate_user_range_for_test(ptr: u64, len: u64, max_len: u64, _writable: bool) -> bool {
    valid_user_address_range(ptr, len, max_len)
}

fn user_slice(ptr: *const u8, len: u64, max_len: u64) -> Option<&'static [u8]> {
    if !validate_user_range(ptr as u64, len, max_len, false) {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(ptr, len as usize) })
}

fn user_slice_mut(ptr: *mut u8, len: u64, max_len: u64) -> Option<&'static mut [u8]> {
    if !validate_user_range(ptr as u64, len, max_len, true) {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts_mut(ptr, len as usize) })
}

// ── jump_to_userspace ─────────────────────────────────────────────────────────

/// Switch the current ring-0 context to ring-3 by pushing a synthetic iretq
/// frame and executing iretq.  Does not return.
///
/// `entry`    — virtual address of the first ring-3 instruction.
/// `user_rsp` — initial ring-3 stack pointer (must be 16-byte aligned).
#[allow(dead_code)]
pub unsafe fn jump_to_userspace(entry: u64, user_rsp: u64) -> ! {
    let user_cs = crate::gdt::user_code_selector().0 as u64;
    let user_ss = crate::gdt::user_data_selector().0 as u64;
    core::arch::asm!(
        "push {ss}",
        "push {rsp}",
        "push {rflags}",
        "push {cs}",
        "push {rip}",
        "iretq",
        ss     = in(reg) user_ss,
        rsp    = in(reg) user_rsp,
        rflags = in(reg) 0x202u64,   // IF=1 (interrupts enabled in ring 3), reserved bit 1
        cs     = in(reg) user_cs,
        rip    = in(reg) entry,
        options(noreturn),
    );
}
