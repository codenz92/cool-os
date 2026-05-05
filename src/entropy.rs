use core::arch::asm;
use core::num::NonZeroU32;
use core::sync::atomic::{AtomicU64, Ordering};

static RDRAND_CALLS: AtomicU64 = AtomicU64::new(0);
static RDRAND_FAILURES: AtomicU64 = AtomicU64::new(0);

const ENTROPY_ERROR: u32 = rand_core::Error::CUSTOM_START;

pub fn has_hardware_rng() -> bool {
    raw_cpuid::CpuId::new()
        .get_feature_info()
        .map(|info| info.has_rdrand())
        .unwrap_or(false)
}

pub fn fill_random(out: &mut [u8]) -> Result<(), rand_core::Error> {
    if !has_hardware_rng() {
        RDRAND_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(entropy_error());
    }

    let mut offset = 0usize;
    while offset < out.len() {
        let word = rdrand_u64().ok_or_else(entropy_error)?;
        RDRAND_CALLS.fetch_add(1, Ordering::Relaxed);
        let bytes = word.to_le_bytes();
        let take = (out.len() - offset).min(bytes.len());
        out[offset..offset + take].copy_from_slice(&bytes[..take]);
        offset += take;
    }
    Ok(())
}

pub fn status_lines() -> alloc::vec::Vec<alloc::string::String> {
    alloc::vec![
        alloc::format!(
            "entropy: rdrand={} calls={} failures={}",
            if has_hardware_rng() { "yes" } else { "no" },
            RDRAND_CALLS.load(Ordering::Relaxed),
            RDRAND_FAILURES.load(Ordering::Relaxed)
        ),
        alloc::string::String::from(
            "tls: HTTPS requires hardware RNG; insecure fallback is disabled"
        ),
    ]
}

fn entropy_error() -> rand_core::Error {
    rand_core::Error::from(NonZeroU32::new(ENTROPY_ERROR).unwrap())
}

fn rdrand_u64() -> Option<u64> {
    for _ in 0..16 {
        let mut value: u64;
        let mut ok: u8;
        unsafe {
            asm!(
                "rdrand {value}",
                "setc {ok}",
                value = out(reg) value,
                ok = out(reg_byte) ok,
                options(nomem, nostack)
            );
        }
        if ok != 0 {
            return Some(value);
        }
    }
    RDRAND_FAILURES.fetch_add(1, Ordering::Relaxed);
    None
}
