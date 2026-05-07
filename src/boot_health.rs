extern crate alloc;

use alloc::{format, string::String, vec::Vec};

pub const BOOT_DIR: &str = "/BOOT";
pub const STATE_PATH: &str = "/BOOT/STATE.TXT";
pub const HISTORY_PATH: &str = "/BOOT/HISTORY.TXT";
pub const LAST_GOOD_PATH: &str = "/BOOT/LAST-GOOD.TXT";

#[derive(Clone)]
struct BootState {
    status: String,
    pending_update: String,
    attempts: u32,
    last_good_tick: u64,
    last_reason: String,
    last_auto_rollback: String,
}

pub fn init() {
    ensure_layout();
    if let Err(err) = begin_boot() {
        crate::klog::log_owned(format!("boot-health: {}", err));
    }
}

pub fn mark_good(reason: &str) {
    ensure_layout();
    let mut state = load_state();
    let accepted_update = if is_none(&state.pending_update) {
        String::from("none")
    } else {
        state.pending_update.clone()
    };
    state.status = String::from("healthy");
    state.pending_update = String::from("none");
    state.attempts = 0;
    state.last_good_tick = crate::interrupts::ticks();
    state.last_reason = String::from(reason);
    let _ = write_state(&state);
    let _ = write_last_good(&state, &accepted_update);
    append_history(
        if is_none(&accepted_update) {
            "mark-good"
        } else {
            "validation-ok"
        },
        &[
            format!("reason={}", reason),
            format!("accepted_update={}", accepted_update),
        ],
    );
    crate::event_bus::emit("boot-health", "healthy", reason);
    let _ = crate::writeback::barrier();
}

pub fn mark_update_pending(update_id: &str) {
    ensure_layout();
    let mut state = load_state();
    state.status = String::from("pending");
    state.pending_update = clean_token(update_id, "manual");
    state.attempts = 0;
    state.last_reason = String::from("update pending validation");
    let _ = write_state(&state);
    append_history(
        "pending-update",
        &[format!("update={}", state.pending_update)],
    );
    crate::event_bus::emit("boot-health", "pending-update", &state.pending_update);
}

pub fn mark_manual_rollback(reason: &str) {
    ensure_layout();
    let mut state = load_state();
    let previous = state.pending_update.clone();
    state.status = String::from("healthy");
    state.pending_update = String::from("none");
    state.attempts = 0;
    state.last_good_tick = crate::interrupts::ticks();
    state.last_reason = String::from(reason);
    let _ = write_state(&state);
    let _ = write_last_good(&state, "none");
    append_history(
        "manual-rollback",
        &[
            format!("previous_update={}", previous),
            format!("reason={}", reason),
        ],
    );
    crate::event_bus::emit("boot-health", "manual-rollback", reason);
    let _ = crate::writeback::barrier();
}

pub fn record_failed_validation(update_id: &str, reason: &str) -> Result<(), &'static str> {
    let update = clean_token(update_id, "manual");
    if update.is_empty() || is_none(&update) {
        return Err("update id required");
    }
    ensure_layout();
    let mut state = load_state();
    state.status = String::from("validating");
    state.pending_update = update;
    state.attempts = state.attempts.max(1);
    state.last_reason = if reason.trim().is_empty() {
        String::from("validation failed before mark-good")
    } else {
        String::from(reason)
    };
    let _ = write_state(&state);
    append_history(
        "validation-failed",
        &[
            format!("update={}", state.pending_update),
            format!("attempts={}", state.attempts),
            format!("reason={}", state.last_reason),
        ],
    );
    crate::event_bus::emit("boot-health", "validation-failed", &state.pending_update);
    let _ = crate::writeback::barrier();
    Ok(())
}

pub fn status_lines() -> Vec<String> {
    ensure_layout();
    let state = load_state();
    alloc::vec![
        format!(
            "layout={} state={} history={} last_good={}",
            BOOT_DIR, STATE_PATH, HISTORY_PATH, LAST_GOOD_PATH
        ),
        format!(
            "status={} pending_update={} attempts={}",
            state.status, state.pending_update, state.attempts
        ),
        format!(
            "last_good_tick={} last_reason={}",
            state.last_good_tick, state.last_reason
        ),
        format!("last_auto_rollback={}", state.last_auto_rollback),
    ]
}

pub fn recovery_lines() -> Vec<String> {
    let state = load_state();
    alloc::vec![
        format!(
            "boot_health status={} pending_update={} attempts={}",
            state.status, state.pending_update, state.attempts
        ),
        format!(
            "last_known_good={} auto_rollback={}",
            LAST_GOOD_PATH, state.last_auto_rollback
        ),
    ]
}

pub fn history_lines() -> Vec<String> {
    let Some(data) = crate::vfs::vfs_kernel_read_file(HISTORY_PATH) else {
        return alloc::vec![format!("history={} missing", HISTORY_PATH)];
    };
    let Ok(text) = core::str::from_utf8(&data) else {
        return alloc::vec![format!("history={} unreadable", HISTORY_PATH)];
    };
    let mut lines = Vec::new();
    for line in text.lines() {
        if !line.trim().is_empty() {
            lines.push(String::from(line));
        }
    }
    if lines.is_empty() {
        lines.push(format!("history={} empty", HISTORY_PATH));
    }
    lines
}

fn begin_boot() -> Result<(), &'static str> {
    let mut state = load_state();
    if should_auto_rollback(&state) {
        let update = state.pending_update.clone();
        append_history(
            "auto-rollback-start",
            &[
                format!("update={}", update),
                format!("attempts={}", state.attempts),
            ],
        );
        match crate::updates::rollback() {
            Ok(()) => {
                state.status = String::from("recovering");
                state.pending_update = String::from("none");
                state.attempts = 0;
                state.last_reason = String::from("auto rollback after failed validation boot");
                state.last_auto_rollback = update.clone();
                let _ = write_state(&state);
                append_history("auto-rollback-ok", &[format!("update={}", update)]);
                crate::event_bus::emit("boot-health", "auto-rollback", &update);
                crate::println!("[boot-health] auto rollback ok id={}", update);
                let _ = crate::writeback::barrier();
                return Ok(());
            }
            Err(err) => {
                state.status = String::from("recovery-failed");
                state.attempts = state.attempts.saturating_add(1);
                state.last_reason = String::from(err);
                let _ = write_state(&state);
                append_history(
                    "auto-rollback-failed",
                    &[format!("update={} error={}", update, err)],
                );
                crate::println!(
                    "[boot-health] auto rollback failed id={} err={}",
                    update,
                    err
                );
                let _ = crate::writeback::barrier();
                return Err(err);
            }
        }
    }

    if state.status == "booting" && is_none(&state.pending_update) && state.attempts > 0 {
        append_history(
            "previous-boot-incomplete",
            &[format!("attempts={}", state.attempts)],
        );
    }

    state.status = if is_none(&state.pending_update) {
        String::from("booting")
    } else {
        String::from("validating")
    };
    state.attempts = state.attempts.saturating_add(1);
    state.last_reason = if is_none(&state.pending_update) {
        String::from("boot started")
    } else {
        String::from("update validation boot started")
    };
    let _ = write_state(&state);
    append_history(
        "boot-start",
        &[
            format!("status={}", state.status),
            format!("pending_update={}", state.pending_update),
            format!("attempts={}", state.attempts),
        ],
    );
    crate::event_bus::emit("boot-health", "boot-start", &state.status);
    let _ = crate::writeback::barrier();
    Ok(())
}

fn should_auto_rollback(state: &BootState) -> bool {
    !is_none(&state.pending_update)
        && state.attempts > 0
        && (state.status == "validating" || state.status == "booting")
}

fn load_state() -> BootState {
    let Some(data) = crate::vfs::vfs_kernel_read_file(STATE_PATH) else {
        return default_state();
    };
    let Ok(text) = core::str::from_utf8(&data) else {
        return default_state();
    };
    let mut state = default_state();
    for raw in text.lines() {
        let Some((key, value)) = raw.trim().split_once('=') else {
            continue;
        };
        match key {
            "status" => state.status = String::from(value),
            "pending_update" => state.pending_update = clean_token(value, "none"),
            "attempts" => state.attempts = value.parse::<u32>().unwrap_or(0),
            "last_good_tick" => state.last_good_tick = value.parse::<u64>().unwrap_or(0),
            "last_reason" => state.last_reason = String::from(value),
            "last_auto_rollback" => state.last_auto_rollback = clean_token(value, "none"),
            _ => {}
        }
    }
    state
}

fn default_state() -> BootState {
    BootState {
        status: String::from("healthy"),
        pending_update: String::from("none"),
        attempts: 0,
        last_good_tick: 0,
        last_reason: String::from("first boot"),
        last_auto_rollback: String::from("none"),
    }
}

fn write_state(state: &BootState) -> Result<(), crate::fat32::FsError> {
    let mut out = String::from("coolOS boot health\n");
    out.push_str(&format!("status={}\n", state.status));
    out.push_str(&format!("pending_update={}\n", state.pending_update));
    out.push_str(&format!("attempts={}\n", state.attempts));
    out.push_str(&format!("last_good_tick={}\n", state.last_good_tick));
    out.push_str(&format!("last_reason={}\n", state.last_reason));
    out.push_str(&format!(
        "last_auto_rollback={}\n",
        state.last_auto_rollback
    ));
    write_file(STATE_PATH, out.as_bytes())
}

fn write_last_good(state: &BootState, accepted_update: &str) -> Result<(), crate::fat32::FsError> {
    let mut out = String::from("coolOS last known good\n");
    out.push_str(&format!("tick={}\n", state.last_good_tick));
    out.push_str(&format!("reason={}\n", state.last_reason));
    out.push_str(&format!("accepted_update={}\n", accepted_update));
    out.push_str(&format!("state={}\n", STATE_PATH));
    write_file(LAST_GOOD_PATH, out.as_bytes())
}

fn append_history(action: &str, details: &[String]) {
    ensure_layout();
    let mut history = match crate::vfs::vfs_kernel_read_file(HISTORY_PATH) {
        Some(bytes) => core::str::from_utf8(&bytes)
            .map(String::from)
            .unwrap_or_else(|_| String::from("coolOS boot health history\n")),
        None => String::from("coolOS boot health history\n"),
    };
    if history.len() > 8192 {
        history = String::from("coolOS boot health history\ntrimmed=true\n");
    }
    history.push_str(&format!(
        "tick={} action={}\n",
        crate::interrupts::ticks(),
        action
    ));
    for detail in details {
        history.push_str(detail);
        history.push('\n');
    }
    let _ = write_file(HISTORY_PATH, history.as_bytes());
}

fn ensure_layout() {
    let _ = crate::vfs::vfs_kernel_create_dir(BOOT_DIR);
    let _ = crate::vfs::vfs_kernel_create_dir("/LOGS");
}

fn write_file(path: &str, data: &[u8]) -> Result<(), crate::fat32::FsError> {
    match crate::vfs::vfs_kernel_create_file(path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
        Err(err) => return Err(err),
    }
    crate::vfs::vfs_kernel_safe_write_file(path, data)
}

fn clean_token(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::from(fallback);
    }
    let mut out = String::new();
    for ch in trimmed.chars() {
        if ch.is_whitespace() || ch == '=' {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    if out.is_empty() {
        String::from(fallback)
    } else {
        out
    }
}

fn is_none(value: &str) -> bool {
    value.is_empty() || value == "none"
}
