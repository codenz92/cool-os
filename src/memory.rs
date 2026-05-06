extern crate alloc;

use alloc::vec::Vec;
use bootloader_api::info::{MemoryRegion, MemoryRegionKind};
use x86_64::{
    structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    &mut *page_table_ptr
}

pub struct BootInfoFrameAllocator {
    memory_regions: &'static [MemoryRegion],
    next: usize,
    recycled: Vec<PhysFrame>,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_regions: &'static [MemoryRegion]) -> Self {
        BootInfoFrameAllocator {
            memory_regions,
            next: 0,
            recycled: Vec::new(),
        }
    }

    /// Start a new allocator that skips the first `start` usable frames,
    /// picking up exactly where a previous allocator with `next == start` left off.
    pub unsafe fn init_from(memory_regions: &'static [MemoryRegion], start: usize) -> Self {
        BootInfoFrameAllocator {
            memory_regions,
            next: start,
            recycled: Vec::new(),
        }
    }

    /// How many frames have been allocated so far.
    pub fn next(&self) -> usize {
        self.next
    }

    pub fn allocate_contiguous(&mut self, count: usize) -> Option<PhysFrame> {
        if count == 0 {
            return None;
        }

        let mut run_start_index = 0usize;
        let mut run_start_frame = None;
        let mut run_len = 0usize;
        let mut last_addr = 0u64;

        let mut found = None;
        {
            for (idx, frame) in self.usable_frames().enumerate().skip(self.next) {
                let addr = frame.start_address().as_u64();
                if run_len == 0 || addr != last_addr + 4096 {
                    run_start_index = idx;
                    run_start_frame = Some(frame);
                    run_len = 1;
                } else {
                    run_len += 1;
                }
                last_addr = addr;

                if run_len == count {
                    found = run_start_frame.map(|frame| (run_start_index + count, frame));
                    break;
                }
            }
        }

        if let Some((next, frame)) = found {
            self.next = next;
            Some(frame)
        } else {
            None
        }
    }

    pub fn deallocate_frame(&mut self, frame: PhysFrame) {
        self.recycled.push(frame);
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> + '_ {
        self.memory_regions
            .iter()
            .filter(|r| r.kind == MemoryRegionKind::Usable)
            .map(|r| r.start..r.end)
            .flat_map(|r| r.step_by(4096))
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        if let Some(frame) = self.recycled.pop() {
            return Some(frame);
        }
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
