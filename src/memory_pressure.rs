extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub const HEAP_LOW_FREE_BYTES: usize = 8 * 1024 * 1024;
pub const HEAP_CRITICAL_FREE_BYTES: usize = 4 * 1024 * 1024;
pub const HEAP_ADMISSION_RESERVE_BYTES: usize = 2 * 1024 * 1024;

const CHECK_INTERVAL_TICKS: u64 = crate::interrupts::TIMER_HZ as u64;
const COOLFS_TRIM_KEEP_BLOCKS: usize = 8;
const OOM_EXIT_CODE: u64 = 137;

static LAST_CHECK_TICK: AtomicU64 = AtomicU64::new(0);
static CHECKS: AtomicUsize = AtomicUsize::new(0);
static TRIM_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);
static COOLFS_TRIMMED_BYTES: AtomicUsize = AtomicUsize::new(0);
static BROWSER_TRIMMED_BYTES: AtomicUsize = AtomicUsize::new(0);
static OOM_KILLS: AtomicUsize = AtomicUsize::new(0);
static LAST_RECLAIMED_BYTES: AtomicUsize = AtomicUsize::new(0);
static LAST_VICTIM_PID: AtomicUsize = AtomicUsize::new(usize::MAX);
static LAST_LEVEL: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PressureLevel {
    Normal,
    Low,
    Critical,
}

#[derive(Clone, Copy)]
pub struct MemoryPressureSnapshot {
    pub heap_total: usize,
    pub heap_used: usize,
    pub heap_free: usize,
    pub heap_high_water: usize,
    pub level: PressureLevel,
    pub checks: usize,
    pub trim_attempts: usize,
    pub coolfs_trimmed_bytes: usize,
    pub browser_trimmed_bytes: usize,
    pub oom_kills: usize,
    pub last_reclaimed_bytes: usize,
    pub last_victim_pid: Option<usize>,
}

impl PressureLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            PressureLevel::Normal => "normal",
            PressureLevel::Low => "low",
            PressureLevel::Critical => "critical",
        }
    }

    fn as_usize(self) -> usize {
        match self {
            PressureLevel::Normal => 0,
            PressureLevel::Low => 1,
            PressureLevel::Critical => 2,
        }
    }
}

pub fn pressure_level(free_bytes: usize, total_bytes: usize) -> PressureLevel {
    let used_bytes = total_bytes.saturating_sub(free_bytes);
    let used_pct = if total_bytes == 0 {
        100
    } else {
        used_bytes.saturating_mul(100) / total_bytes
    };
    if free_bytes <= HEAP_CRITICAL_FREE_BYTES || used_pct >= 90 {
        PressureLevel::Critical
    } else if free_bytes <= HEAP_LOW_FREE_BYTES || used_pct >= 75 {
        PressureLevel::Low
    } else {
        PressureLevel::Normal
    }
}

pub fn snapshot() -> MemoryPressureSnapshot {
    let heap = crate::allocator::heap_snapshot();
    let level = pressure_level(heap.free, heap.total);
    LAST_LEVEL.store(level.as_usize(), Ordering::Relaxed);
    let last_victim = LAST_VICTIM_PID.load(Ordering::Relaxed);
    MemoryPressureSnapshot {
        heap_total: heap.total,
        heap_used: heap.used,
        heap_free: heap.free,
        heap_high_water: heap.high_water,
        level,
        checks: CHECKS.load(Ordering::Relaxed),
        trim_attempts: TRIM_ATTEMPTS.load(Ordering::Relaxed),
        coolfs_trimmed_bytes: COOLFS_TRIMMED_BYTES.load(Ordering::Relaxed),
        browser_trimmed_bytes: BROWSER_TRIMMED_BYTES.load(Ordering::Relaxed),
        oom_kills: OOM_KILLS.load(Ordering::Relaxed),
        last_reclaimed_bytes: LAST_RECLAIMED_BYTES.load(Ordering::Relaxed),
        last_victim_pid: if last_victim == usize::MAX {
            None
        } else {
            Some(last_victim)
        },
    }
}

pub fn tick() {
    let now = crate::interrupts::ticks();
    let last = LAST_CHECK_TICK.load(Ordering::Relaxed);
    if now.wrapping_sub(last) < CHECK_INTERVAL_TICKS {
        return;
    }
    if LAST_CHECK_TICK
        .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        let _ = check_and_recover("periodic");
    }
}

pub fn admit_allocation(bytes: usize, trigger: &'static str) -> bool {
    if bytes == 0 {
        return true;
    }
    if has_allocation_reserve(bytes) {
        return true;
    }

    let after = check_and_recover(trigger);
    after.heap_free.saturating_sub(bytes) >= HEAP_ADMISSION_RESERVE_BYTES
}

pub fn has_allocation_reserve(bytes: usize) -> bool {
    snapshot().heap_free.saturating_sub(bytes) >= HEAP_ADMISSION_RESERVE_BYTES
}

pub fn check_and_recover(_trigger: &'static str) -> MemoryPressureSnapshot {
    CHECKS.fetch_add(1, Ordering::Relaxed);
    let before = snapshot();
    if before.level >= PressureLevel::Low {
        let reclaimed = trim_reclaimable();
        if reclaimed > 0 {
            crate::klog::log("memory pressure: trimmed reclaimable caches");
        }
    }

    let after_trim = snapshot();
    if after_trim.level == PressureLevel::Critical {
        if let Some(victim) = crate::scheduler::reclaim_largest_user_task(OOM_EXIT_CODE) {
            OOM_KILLS.fetch_add(1, Ordering::Relaxed);
            LAST_RECLAIMED_BYTES.store(victim.estimated_bytes, Ordering::Relaxed);
            LAST_VICTIM_PID.store(victim.pid, Ordering::Relaxed);
            crate::klog::log("memory pressure: oom reclaimed user task");
        }
    }

    snapshot()
}

fn trim_reclaimable() -> usize {
    TRIM_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    let _ = crate::fs_hardening::flush();
    let coolfs = crate::coolfs::trim_clean_cache(COOLFS_TRIM_KEEP_BLOCKS).unwrap_or(0);
    let browser = crate::wm::trim_browser_memory_pressure();
    if coolfs > 0 {
        COOLFS_TRIMMED_BYTES.fetch_add(coolfs, Ordering::Relaxed);
    }
    if browser > 0 {
        BROWSER_TRIMMED_BYTES.fetch_add(browser, Ordering::Relaxed);
    }
    coolfs.saturating_add(browser)
}

pub fn lines() -> Vec<String> {
    let snap = snapshot();
    let victim = snap
        .last_victim_pid
        .map(|pid| format!("{}", pid))
        .unwrap_or_else(|| String::from("none"));
    vec![
        format!(
            "heap pressure={} used={} free={} total={} high_water={}",
            snap.level.as_str(),
            snap.heap_used,
            snap.heap_free,
            snap.heap_total,
            snap.heap_high_water
        ),
        format!(
            "thresholds low_free={} critical_free={} admission_reserve={}",
            HEAP_LOW_FREE_BYTES, HEAP_CRITICAL_FREE_BYTES, HEAP_ADMISSION_RESERVE_BYTES
        ),
        format!(
            "reclaim checks={} trim_attempts={} coolfs_trimmed={} browser_trimmed={}",
            snap.checks, snap.trim_attempts, snap.coolfs_trimmed_bytes, snap.browser_trimmed_bytes
        ),
        format!(
            "oom kills={} last_victim={} last_reclaimed_est={}",
            snap.oom_kills, victim, snap.last_reclaimed_bytes
        ),
    ]
}

pub fn selftest_passes() -> bool {
    pressure_level(HEAP_LOW_FREE_BYTES + 4096, crate::allocator::HEAP_SIZE) == PressureLevel::Normal
        && pressure_level(HEAP_LOW_FREE_BYTES, crate::allocator::HEAP_SIZE) == PressureLevel::Low
        && pressure_level(HEAP_CRITICAL_FREE_BYTES, crate::allocator::HEAP_SIZE)
            == PressureLevel::Critical
        && HEAP_CRITICAL_FREE_BYTES < HEAP_LOW_FREE_BYTES
        && HEAP_ADMISSION_RESERVE_BYTES <= HEAP_CRITICAL_FREE_BYTES
}
