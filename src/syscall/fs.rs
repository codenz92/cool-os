fn sys_fs_write_file(desc_ptr: *const u8) -> u64 {
    let Some(desc) = user_descriptor4(desc_ptr) else {
        return u64::MAX;
    };
    let Some(path) = user_path(desc[0] as *const u8, desc[1]) else {
        return u64::MAX;
    };
    let data = if desc[3] == 0 {
        &[]
    } else {
        let Some(data) = user_slice(desc[2] as *const u8, desc[3], MAX_USER_BUFFER) else {
            return u64::MAX;
        };
        data
    };

    let path = resolve_task_path(path);
    match crate::vfs::vfs_create_file(&path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
        Err(_) => return u64::MAX,
    }

    match crate::vfs::vfs_write_file(&path, data) {
        Ok(()) => 0,
        Err(_) => u64::MAX,
    }
}

fn sys_fs_create_dir(path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(path) = user_path(path_ptr, path_len) else {
        return u64::MAX;
    };
    let path = resolve_task_path(path);
    match crate::vfs::vfs_create_dir(&path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => 0,
        Err(_) => u64::MAX,
    }
}

fn sys_fs_delete_tree(path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(path) = user_path(path_ptr, path_len) else {
        return u64::MAX;
    };
    let path = resolve_task_path(path);
    match crate::vfs::vfs_delete_recursive(&path) {
        Ok(()) => 0,
        Err(_) => u64::MAX,
    }
}

fn sys_fs_list_dir(desc_ptr: *const u8) -> u64 {
    let Some(desc) = user_descriptor4(desc_ptr) else {
        return u64::MAX;
    };
    let Some(path) = user_path(desc[0] as *const u8, desc[1]) else {
        return u64::MAX;
    };
    let Some(out) = user_slice_mut(desc[2] as *mut u8, desc[3], MAX_USER_DIR_LISTING) else {
        return u64::MAX;
    };
    let path = resolve_task_path(path);
    let Some(entries) = crate::vfs::vfs_list_dir(&path) else {
        return u64::MAX;
    };

    let mut written = 0usize;
    for entry in entries {
        if !append_dir_listing_byte(out, &mut written, if entry.is_dir { b'D' } else { b'F' }) {
            break;
        }
        if !append_dir_listing_byte(out, &mut written, b'\t') {
            break;
        }
        if !append_dir_listing_bytes(out, &mut written, entry.name.as_bytes()) {
            break;
        }
        if !append_dir_listing_byte(out, &mut written, b'\t') {
            break;
        }
        if !append_dir_listing_u64(out, &mut written, entry.size as u64) {
            break;
        }
        if !append_dir_listing_byte(out, &mut written, b'\n') {
            break;
        }
    }
    written as u64
}

fn sys_screenshot(path_ptr: *const u8, path_len: u64, _flags: u64) -> u64 {
    if !crate::security::can_desktop(crate::security::current_credentials()) {
        return u64::MAX;
    }
    let Some(path) = user_path(path_ptr, path_len) else {
        return u64::MAX;
    };
    let path = resolve_task_path(path);
    crate::wm::request_focused_screenshot(&path);
    0
}

fn sys_waitpid(pid: u64, status_ptr: *mut u64) -> u64 {
    if !status_ptr.is_null() && !validate_user_range(status_ptr as u64, 8, 8, true) {
        return u64::MAX;
    }
    let parent = crate::scheduler::current_task_id();
    loop {
        match crate::scheduler::waitpid(parent, pid as usize) {
            Ok(code) => unsafe {
                if !status_ptr.is_null() {
                    *status_ptr = code;
                }
                return pid;
            },
            Err(crate::scheduler::WaitError::NotExited) => {
                crate::wait_queue::wait("waitpid", parent);
                crate::scheduler::block_current();
                while crate::scheduler::current_task_blocked() {
                    if crate::scheduler::current_has_pending_signal() {
                        break;
                    }
                    unsafe {
                        core::arch::asm!("sti; hlt; cli", options(nomem, nostack));
                    }
                }
                crate::wait_queue::wake("waitpid", parent);
                x86_64::instructions::interrupts::disable();
                if crate::scheduler::current_has_pending_signal() {
                    return u64::MAX;
                }
            }
            Err(_) => return u64::MAX,
        }
    }
}

fn sys_spawn(path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(bytes) = user_slice(path_ptr, path_len, MAX_USER_STRING) else {
        return u64::MAX;
    };
    let path = match core::str::from_utf8(bytes) {
        Ok(path) => path,
        Err(_) => return u64::MAX,
    };
    let path = resolve_task_path(path);
    match crate::elf::spawn_elf_process_with_args(&path, &[]) {
        Ok(pid) => pid as u64,
        Err(_) => u64::MAX,
    }
}

fn sys_spawn_args(desc_ptr: *const u8) -> u64 {
    const MAX_ARGC: u64 = 7;
    let Some(desc) = user_descriptor4(desc_ptr) else {
        return u64::MAX;
    };
    let Some(path) = user_path(desc[0] as *const u8, desc[1]) else {
        return u64::MAX;
    };
    let argc = desc[3];
    if argc > MAX_ARGC {
        return u64::MAX;
    }

    let arg_pairs = if argc == 0 {
        &[][..]
    } else {
        let pair_bytes = argc.saturating_mul(16);
        let Some(bytes) = user_slice(desc[2] as *const u8, pair_bytes, MAX_ARGC * 16) else {
            return u64::MAX;
        };
        bytes
    };

    let mut arg_strings = Vec::new();
    if arg_strings.try_reserve_exact(argc as usize).is_err() {
        return u64::MAX;
    }
    for idx in 0..argc as usize {
        let base = idx * 16;
        let ptr = u64::from_le_bytes(match arg_pairs[base..base + 8].try_into() {
            Ok(bytes) => bytes,
            Err(_) => return u64::MAX,
        });
        let len = u64::from_le_bytes(match arg_pairs[base + 8..base + 16].try_into() {
            Ok(bytes) => bytes,
            Err(_) => return u64::MAX,
        });
        let Some(bytes) = user_slice(ptr as *const u8, len, MAX_USER_STRING) else {
            return u64::MAX;
        };
        let Ok(arg) = core::str::from_utf8(bytes) else {
            return u64::MAX;
        };
        arg_strings.push(String::from(arg));
    }

    let path = resolve_task_path(path);
    let mut arg_refs = Vec::new();
    if arg_refs.try_reserve_exact(arg_strings.len()).is_err() {
        return u64::MAX;
    }
    for arg in &arg_strings {
        arg_refs.push(arg.as_str());
    }

    match crate::elf::spawn_elf_process_with_args(&path, &arg_refs) {
        Ok(pid) => pid as u64,
        Err(_) => u64::MAX,
    }
}

fn sys_chdir(path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(path) = user_path(path_ptr, path_len) else {
        return u64::MAX;
    };
    let path = resolve_task_path(path);
    if crate::vfs::vfs_list_dir(&path).is_none() {
        return u64::MAX;
    }
    crate::scheduler::set_current_cwd(path);
    0
}

fn sys_getcwd(buf_ptr: *mut u8, len: u64) -> u64 {
    if len == 0 || !validate_user_range(buf_ptr as u64, len, MAX_USER_STRING, true) {
        return u64::MAX;
    }
    let cwd = crate::scheduler::current_cwd();
    let bytes = cwd.as_bytes();
    if bytes.len() > len as usize {
        return u64::MAX;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), buf_ptr, bytes.len());
    }
    bytes.len() as u64
}

fn sys_stat(desc_ptr: *const u8) -> u64 {
    let Some(desc) = user_descriptor4(desc_ptr) else {
        return u64::MAX;
    };
    let Some(path) = user_path(desc[0] as *const u8, desc[1]) else {
        return u64::MAX;
    };
    if desc[3] < STAT_RECORD_BYTES {
        return u64::MAX;
    }
    let Some(out) = user_slice_mut(desc[2] as *mut u8, STAT_RECORD_BYTES, STAT_RECORD_BYTES) else {
        return u64::MAX;
    };
    let path = resolve_task_path(path);
    let Some(meta) = crate::vfs::vfs_metadata(&path) else {
        return u64::MAX;
    };
    let kind = if meta.is_dir {
        2u64
    } else if meta.is_file {
        1u64
    } else {
        0u64
    };
    write_record_u64(out, 0, kind);
    write_record_u64(out, 8, meta.size);
    write_record_u64(out, 16, meta.uid as u64);
    write_record_u64(out, 24, meta.gid as u64);
    write_record_u64(out, 32, meta.mode as u64);
    STAT_RECORD_BYTES
}

fn sys_rename(desc_ptr: *const u8) -> u64 {
    let Some(desc) = user_descriptor4(desc_ptr) else {
        return u64::MAX;
    };
    let Some(src) = user_path(desc[0] as *const u8, desc[1]) else {
        return u64::MAX;
    };
    let Some(dst) = user_path(desc[2] as *const u8, desc[3]) else {
        return u64::MAX;
    };
    let src = resolve_task_path(src);
    let dst = resolve_task_path(dst);
    match crate::vfs::vfs_rename_path(&src, &dst) {
        Ok(()) => 0,
        Err(_) => u64::MAX,
    }
}

fn sys_open_write(path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(path) = user_path(path_ptr, path_len) else {
        return u64::MAX;
    };
    let path = resolve_task_path(path);
    let fd = crate::vfs::vfs_open_write(&path);
    if fd == usize::MAX {
        u64::MAX
    } else {
        fd as u64
    }
}

fn sys_spawn_fds_args(desc_ptr: *const u8) -> u64 {
    const MAX_ARGC: u64 = 7;
    const MAX_FD_MAPS: u64 = 4;
    let Some(desc) = user_descriptor6(desc_ptr) else {
        return u64::MAX;
    };
    let Some(path) = user_path(desc[0] as *const u8, desc[1]) else {
        return u64::MAX;
    };
    let argc = desc[3];
    let fd_count = desc[5];
    if argc > MAX_ARGC || fd_count > MAX_FD_MAPS {
        return u64::MAX;
    }

    let Some(arg_strings) = parse_user_arg_strings(desc[2], argc, MAX_ARGC) else {
        return u64::MAX;
    };
    let Some(fd_mappings) = parse_user_fd_mappings(desc[4], fd_count, MAX_FD_MAPS) else {
        return u64::MAX;
    };

    let mut arg_refs = Vec::new();
    if arg_refs.try_reserve_exact(arg_strings.len()).is_err() {
        return u64::MAX;
    }
    for arg in &arg_strings {
        arg_refs.push(arg.as_str());
    }

    let path = resolve_task_path(path);
    match crate::elf::spawn_elf_process_with_fds(&path, &arg_refs, &fd_mappings) {
        Ok(pid) => pid as u64,
        Err(_) => u64::MAX,
    }
}

fn sys_sync() -> u64 {
    match crate::writeback::barrier() {
        Ok(()) => 0,
        Err(_) => u64::MAX,
    }
}
