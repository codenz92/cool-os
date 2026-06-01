fn mirror_debug_char(c: char) {
    if !DEBUG_MIRROR.load(Ordering::Acquire) {
        return;
    }
    if c == '\n' {
        debug_byte(b'\n');
    } else if c.is_ascii() && !c.is_control() {
        debug_byte(c as u8);
    } else if !c.is_control() {
        debug_byte(b'?');
    }
}

fn debug_byte(byte: u8) {
    unsafe {
        x86_64::instructions::port::Port::<u8>::new(0xE9).write(byte);
    }
}

fn ctrl_byte(key: Key) -> Option<u8> {
    match key {
        Key::Character(c) if c.is_ascii_alphabetic() => Some((c.to_ascii_uppercase() as u8) - b'@'),
        Key::Character('[') | Key::Escape => Some(0x1b),
        Key::Character('\\') => Some(0x1c),
        Key::Character(']') => Some(0x1d),
        Key::Character('^') => Some(0x1e),
        Key::Character('_') => Some(0x1f),
        Key::Space => Some(0x00),
        Key::Backspace => Some(0x08),
        Key::Delete => Some(0x7f),
        _ => None,
    }
}

// ── Path utilities ────────────────────────────────────────────────────────────

fn text_cols(width: usize) -> usize {
    (width.saturating_sub(TERM_PAD_X * 2) / CHAR_W).max(1)
}

fn text_rows(content_h: usize) -> usize {
    (content_h.saturating_sub(TERM_PAD_Y * 2) / LINE_H).max(1)
}

fn resolve_path(cwd: &str, input: &str) -> String {
    if input.starts_with('/') {
        normalize_path(input)
    } else {
        let mut base = String::from(cwd);
        if !base.ends_with('/') {
            base.push('/');
        }
        base.push_str(input);
        normalize_path(&base)
    }
}

fn parse_usize(input: &str) -> Option<usize> {
    if input.is_empty() {
        return None;
    }
    let mut out = 0usize;
    for b in input.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        out = out.checked_mul(10)?.checked_add((b - b'0') as usize)?;
    }
    Some(out)
}

fn parse_u32(input: &str) -> Option<u32> {
    if input.is_empty() {
        return None;
    }
    let mut out = 0u32;
    for b in input.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        out = out.checked_mul(10)?.checked_add((b - b'0') as u32)?;
    }
    Some(out)
}

fn parse_u64(input: &str) -> Option<u64> {
    if input.is_empty() {
        return None;
    }
    let mut out = 0u64;
    for b in input.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        out = out.checked_mul(10)?.checked_add((b - b'0') as u64)?;
    }
    Some(out)
}

fn parse_job_id(input: &str) -> Option<u64> {
    match input {
        "last" | "latest" => crate::jobs::latest_id(),
        _ => parse_u64(input),
    }
}

fn parse_owner(input: &str) -> Option<(u32, u32)> {
    if let Some((uid, gid)) = input.split_once(':') {
        return Some((parse_u32(uid)?, parse_u32(gid)?));
    }
    let uid = parse_u32(input)?;
    Some((uid, uid))
}

fn parse_bool_word(input: &str) -> Option<bool> {
    match input {
        "on" | "1" | "true" | "yes" => Some(true),
        "off" | "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

fn resolve_path_if_archive(cwd: &str, value: &str) -> String {
    if value.starts_with('/') || value.to_ascii_uppercase().ends_with(".PKG") {
        resolve_path(cwd, value)
    } else {
        String::from(value)
    }
}

fn collect_words<'a, I>(words: I) -> String
where
    I: Iterator<Item = &'a str>,
{
    let mut out = String::new();
    for word in words {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

fn diagnostics_lines() -> Vec<String> {
    let mut lines = Vec::new();
    push_terminal_section(&mut lines, "kernel", crate::klog::lines());
    push_terminal_section(&mut lines, "profiler", crate::profiler::lines());
    push_terminal_section(
        &mut lines,
        "boot health",
        crate::boot_health::status_lines(),
    );
    push_terminal_section(&mut lines, "hardware", crate::hardware::lines());
    push_terminal_section(&mut lines, "services", crate::services::lines());
    push_terminal_section(&mut lines, "updates", crate::updates::status_lines());
    push_terminal_section(&mut lines, "packages", crate::packages::status_lines());
    push_terminal_section(
        &mut lines,
        "browser engine",
        crate::browser_engine::status_lines(),
    );
    push_terminal_section(
        &mut lines,
        "compositor",
        crate::wm::compositor::compositor_lines(),
    );
    push_terminal_section(&mut lines, "heap", crate::allocator::heap_lines());
    push_terminal_section(
        &mut lines,
        "memory pressure",
        crate::memory_pressure::lines(),
    );
    push_terminal_section(
        &mut lines,
        "task memory",
        crate::scheduler::task_memory_lines(),
    );
    push_terminal_section(
        &mut lines,
        "resource limits",
        crate::resource_limits::lines(),
    );
    push_terminal_section(&mut lines, "futex", crate::futex::lines());
    push_terminal_section(&mut lines, "slab", crate::slab::lines());
    push_terminal_section(
        &mut lines,
        "filesystem",
        crate::fs_hardening::status_lines(),
    );
    push_terminal_section(&mut lines, "vfs", crate::vfs::mount_lines());
    push_terminal_section(&mut lines, "config", crate::config_store::lines());
    push_terminal_section(&mut lines, "settings", crate::settings_state::lines());
    push_terminal_section(&mut lines, "crash", crate::crashdump::detailed_lines());
    lines
}

fn push_terminal_section(out: &mut Vec<String>, name: &str, lines: Vec<String>) {
    let mut header = String::from("== ");
    header.push_str(name);
    header.push_str(" ==");
    out.push(header);
    if lines.is_empty() {
        out.push(String::from("(none)"));
    } else {
        out.extend(lines);
    }
}

fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for component in path.split('/').filter(|s| !s.is_empty()) {
        match component {
            ".." => {
                parts.pop();
            }
            "." => {}
            seg => parts.push(seg),
        }
    }
    if parts.is_empty() {
        return String::from("/");
    }
    let mut result = String::from("/");
    for (i, &part) in parts.iter().enumerate() {
        if i > 0 {
            result.push('/');
        }
        result.push_str(part);
    }
    result
}
