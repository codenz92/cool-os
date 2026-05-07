extern crate alloc;

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use linked_list_allocator::LockedHeap;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();
static HEAP_READY: AtomicBool = AtomicBool::new(false);
static HIGH_WATER: AtomicUsize = AtomicUsize::new(0);
static DIAG_SAMPLES: AtomicUsize = AtomicUsize::new(0);

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 32 * 1024 * 1024; // 32 MiB

#[derive(Clone, Copy)]
pub struct HeapSnapshot {
    pub total: usize,
    pub used: usize,
    pub free: usize,
    pub high_water: usize,
    pub diag_samples: usize,
}

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }
    HEAP_READY.store(true, Ordering::Release);
    Ok(())
}

fn record_heap_sample(used: usize) {
    DIAG_SAMPLES.fetch_add(1, Ordering::Relaxed);
    let mut high = HIGH_WATER.load(Ordering::Relaxed);
    while used > high {
        match HIGH_WATER.compare_exchange_weak(high, used, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(next) => high = next,
        }
    }
}

pub fn heap_snapshot() -> HeapSnapshot {
    let used = ALLOCATOR.lock().used();
    record_heap_sample(used);
    HeapSnapshot {
        total: HEAP_SIZE,
        used,
        free: HEAP_SIZE.saturating_sub(used),
        high_water: HIGH_WATER.load(Ordering::Relaxed),
        diag_samples: DIAG_SAMPLES.load(Ordering::Relaxed),
    }
}

pub fn heap_ready() -> bool {
    HEAP_READY.load(Ordering::Acquire)
}

pub fn heap_lines() -> alloc::vec::Vec<alloc::string::String> {
    let snapshot = heap_snapshot();
    alloc::vec![
        alloc::format!("heap_total={} bytes", snapshot.total),
        alloc::format!("heap_used={} bytes", snapshot.used),
        alloc::format!("heap_free={} bytes", snapshot.free),
        alloc::format!("heap_high_water={} bytes", snapshot.high_water),
        alloc::format!("allocator_diag_samples={}", snapshot.diag_samples),
        alloc::format!("fragmentation_probe=free/used/high-water via linked-list allocator"),
    ]
}
