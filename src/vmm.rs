/// Virtual Memory Manager (Phase 10/32).
///
/// Provides a globally-accessible frame allocator and helpers for building and
/// switching per-process PML4 page tables.
///
/// Address-space layout:
///   L4 index 0x80             — per-process shared-memory windows
///   L4 index 0xFF             — per-process ELF image, mmap arena, user stack
///   all other present entries — shared kernel mappings, supervisor-only
///
/// coolOS still runs a lower-half kernel, so process PML4s keep kernel entries
/// present for syscall/interrupt execution. Phase 32 makes those entries
/// supervisor-only instead of user-accessible.
extern crate alloc;

use alloc::vec::Vec;
use spin::Mutex;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame,
        Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use crate::memory::BootInfoFrameAllocator;

// ── Globals ───────────────────────────────────────────────────────────────────

static PHYS_OFFSET: Mutex<u64> = Mutex::new(0);
static FRAME_ALLOC: Mutex<Option<BootInfoFrameAllocator>> = Mutex::new(None);
static BOOT_PML4: Mutex<u64> = Mutex::new(0);
static ADDRESS_SPACES: Mutex<Vec<AddressSpaceFrames>> = Mutex::new(Vec::new());

struct AddressSpaceFrames {
    pml4: PhysFrame,
    table_frames: Vec<PhysFrame>,
    leaf_frames: Vec<PhysFrame>,
}

#[derive(Clone, Copy)]
pub struct VmmResourceStats {
    pub address_spaces: usize,
    pub page_table_pages: usize,
    pub owned_leaf_pages: usize,
}

struct TrackingFrameAllocator<'a> {
    inner: &'a mut BootInfoFrameAllocator,
    allocated: Vec<PhysFrame>,
}

/// Per-process user stack: 64 KiB ending at this virtual address.
pub const USER_STACK_TOP: u64 = 0x0000_7fff_0010_0000;
/// Size of the per-process user stack (64 KiB).
pub const USER_STACK_SIZE: u64 = 64 * 1024;
/// Bottom of the user stack (guard page sits just below this).
pub const USER_STACK_BOTTOM: u64 = USER_STACK_TOP - USER_STACK_SIZE;
/// Exclusive upper bound for canonical user addresses.
pub const USER_TOP: u64 = 0x0000_8000_0000_0000;
/// Base of the userspace ELF/linker region.
pub const USER_IMAGE_BASE: u64 = 0x0000_7fff_0000_0000;
/// Explicit mmap arena. User processes may not mmap arbitrary lower-half pages.
pub const USER_MMAP_BASE: u64 = 0x0000_7fff_1000_0000;
/// Exclusive top of the explicit mmap arena.
pub const USER_MMAP_TOP: u64 = 0x0000_7fff_7000_0000;
/// Shared-memory windows start in their own PML4 root.
pub const USER_SHMEM_BASE: u64 = 0x0000_4000_0000_0000;
/// Per-shmem-id virtual slot size.
pub const USER_SHMEM_SLOT_SIZE: u64 = 64 * 1024 * 1024;

const USER_IMAGE_PML4_INDEX: usize = pml4_index(USER_IMAGE_BASE);
const USER_SHMEM_PML4_INDEX: usize = pml4_index(USER_SHMEM_BASE);

// ── Init ──────────────────────────────────────────────────────────────────────

pub fn init(phys_offset: VirtAddr, alloc: BootInfoFrameAllocator) {
    *PHYS_OFFSET.lock() = phys_offset.as_u64();
    *BOOT_PML4.lock() = Cr3::read().0.start_address().as_u64();
    *FRAME_ALLOC.lock() = Some(alloc);
}

// ── Internal helpers ──────────────────────────────────────────────────────────

pub fn phys_offset() -> VirtAddr {
    VirtAddr::new(*PHYS_OFFSET.lock())
}

/// Convert a physical address to its virtual alias via the physical-memory map.
pub fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    phys_offset() + phys.as_u64()
}

/// Borrow the physical frame at `phys` as a mutable `PageTable` reference.
///
/// # Safety
/// `phys` must be the start of a valid, 4 KiB-aligned page table frame.
unsafe fn table_at(phys: PhysAddr) -> &'static mut PageTable {
    &mut *(phys_to_virt(phys).as_mut_ptr())
}

/// Allocate one 4 KiB physical frame from the global allocator.
pub fn alloc_frame() -> Option<PhysFrame> {
    FRAME_ALLOC.lock().as_mut()?.allocate_frame()
}

/// Allocate a zeroed 4 KiB physical frame.
pub fn alloc_zeroed_frame() -> Option<PhysFrame> {
    let frame = alloc_frame()?;
    let ptr = phys_to_virt(frame.start_address()).as_mut_ptr::<u8>();
    unsafe { core::ptr::write_bytes(ptr, 0, 4096) };
    crate::slab::record_alloc("frames", 4096);
    Some(frame)
}

/// Return a frame to the VMM allocator. The caller must guarantee the frame is
/// no longer mapped anywhere that can be reached.
pub fn free_unmapped_frame(frame: PhysFrame) {
    if let Some(alloc) = FRAME_ALLOC.lock().as_mut() {
        alloc.deallocate_frame(frame);
        crate::slab::record_free("frames", 4096);
    }
}

/// Allocate a physically contiguous run of zeroed frames.
///
/// Legacy virtio queues are handed to the device as one physical page-frame
/// number, so the descriptor/available/used ring memory must be contiguous.
/// The boot allocator yields QEMU's usable memory in ascending frame order; this
/// helper verifies that assumption before exposing the run to a DMA device.
pub fn alloc_contiguous_zeroed_frames(count: usize) -> Option<PhysFrame> {
    if count == 0 {
        return None;
    }
    let first = FRAME_ALLOC.lock().as_mut()?.allocate_contiguous(count)?;
    for idx in 0..count {
        let phys = PhysAddr::new(first.start_address().as_u64() + idx as u64 * 4096);
        let ptr = phys_to_virt(phys).as_mut_ptr::<u8>();
        unsafe { core::ptr::write_bytes(ptr, 0, 4096) };
    }
    crate::slab::record_alloc("frames", count * 4096);
    Some(first)
}

impl<'a> TrackingFrameAllocator<'a> {
    fn new(inner: &'a mut BootInfoFrameAllocator) -> Self {
        Self {
            inner,
            allocated: Vec::new(),
        }
    }

    fn into_allocated(self) -> Vec<PhysFrame> {
        self.allocated
    }
}

unsafe impl<'a> FrameAllocator<Size4KiB> for TrackingFrameAllocator<'a> {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.inner.allocate_frame()?;
        let ptr = phys_to_virt(frame.start_address()).as_mut_ptr::<u8>();
        unsafe { core::ptr::write_bytes(ptr, 0, 4096) };
        crate::slab::record_alloc("frames", 4096);
        self.allocated.push(frame);
        Some(frame)
    }
}

// ── Public VMM API ────────────────────────────────────────────────────────────

/// Harden the boot PML4 by clearing U/S on every present boot mapping.
///
/// This is called before any userspace is spawned. Per-process userspace
/// mappings are created later under fresh PML4 roots and keep U/S set there.
pub fn harden_boot_mappings() {
    let boot_phys = PhysAddr::new(*BOOT_PML4.lock());
    let boot_l4_frame: PhysFrame = PhysFrame::containing_address(boot_phys);
    let l4 = unsafe { table_at(boot_l4_frame.start_address()) };
    clear_user_accessible_recursive(l4, 4);
    x86_64::instructions::tlb::flush_all();
}

/// Allocate a new PML4. Kernel mappings are copied in as supervisor-only so
/// ring 0 can continue to handle syscalls/IRQs after CR3 switches. The explicit
/// user roots are left empty and populated by the ELF loader, mmap, and shmem.
pub fn new_process_pml4() -> Option<PhysFrame> {
    let frame = alloc_zeroed_frame()?;

    let boot_phys = PhysAddr::new(*BOOT_PML4.lock());
    let boot_l4_frame: PhysFrame = PhysFrame::containing_address(boot_phys);
    let src = unsafe { table_at(boot_l4_frame.start_address()) };
    let dst = unsafe { table_at(frame.start_address()) };

    for i in 0..512 {
        if i == USER_IMAGE_PML4_INDEX || i == USER_SHMEM_PML4_INDEX {
            continue;
        }
        let mut entry = src[i].clone();
        if !entry.is_unused() {
            entry.set_flags(entry.flags() & !PageTableFlags::USER_ACCESSIBLE);
        }
        dst[i] = entry;
    }

    ADDRESS_SPACES.lock().push(AddressSpaceFrames {
        pml4: frame,
        table_frames: Vec::new(),
        leaf_frames: Vec::new(),
    });

    Some(frame)
}

/// Map `phys_frame` at virtual address `virt` inside the address space rooted
/// at `pml4_frame`, using the provided page-table flags.  Allocates intermediate
/// page-table frames as needed.
pub fn map_page_in(
    pml4_frame: PhysFrame,
    virt: VirtAddr,
    phys_frame: PhysFrame,
    flags: PageTableFlags,
) -> Result<(), &'static str> {
    let pml4 = unsafe { table_at(pml4_frame.start_address()) };
    let offset = phys_offset();
    let mut mapper = unsafe { OffsetPageTable::new(pml4, offset) };

    let page: Page<Size4KiB> = Page::containing_address(virt);

    let table_frames = {
        let mut guard = FRAME_ALLOC.lock();
        let alloc = guard.as_mut().ok_or("frame allocator not initialized")?;
        let mut tracking_alloc = TrackingFrameAllocator::new(alloc);

        unsafe {
            mapper
                .map_to(page, phys_frame, flags, &mut tracking_alloc)
                .map_err(|_| "map_to failed")?
                .flush();
        }
        tracking_alloc.into_allocated()
    };

    record_table_frames(pml4_frame, table_frames);
    Ok(())
}

/// Map a leaf frame owned by `pml4_frame`. Owned leaf frames are reclaimed when
/// the address space is reaped or replaced by exec.
pub fn map_owned_frame_in(
    pml4_frame: PhysFrame,
    virt: VirtAddr,
    phys_frame: PhysFrame,
    flags: PageTableFlags,
) -> Result<(), &'static str> {
    if !can_add_owned_pages(pml4_frame, 1) {
        return Err("address space resource limit");
    }
    map_page_in(pml4_frame, virt, phys_frame, flags)?;
    record_leaf_frame(pml4_frame, phys_frame);
    Ok(())
}

/// Map `len` bytes of freshly-allocated frames starting at `virt` inside the
/// address space rooted at `pml4_frame`.
pub fn map_region(
    pml4_frame: PhysFrame,
    virt: VirtAddr,
    len: u64,
    flags: PageTableFlags,
) -> Result<(), &'static str> {
    let pages = len.saturating_add(4095).saturating_div(4096) as usize;
    if !can_add_owned_pages(pml4_frame, pages) {
        return Err("address space resource limit");
    }
    let mut offset = 0u64;
    while offset < len {
        let frame = alloc_zeroed_frame().ok_or("out of frames")?;
        if let Err(err) = map_owned_frame_in(pml4_frame, virt + offset, frame, flags) {
            free_unmapped_frame(frame);
            return Err(err);
        }
        offset += 4096;
    }
    Ok(())
}

pub fn owned_leaf_pages(pml4_frame: PhysFrame) -> usize {
    let spaces = ADDRESS_SPACES.lock();
    spaces
        .iter()
        .find(|space| space.pml4 == pml4_frame)
        .map(|space| space.leaf_frames.len())
        .unwrap_or(0)
}

pub fn can_add_owned_pages(pml4_frame: PhysFrame, additional_pages: usize) -> bool {
    owned_leaf_pages(pml4_frame).saturating_add(additional_pages)
        <= crate::resource_limits::MAX_USER_ADDRESS_SPACE_PAGES
}

pub fn resource_stats() -> VmmResourceStats {
    let spaces = ADDRESS_SPACES.lock();
    let mut page_table_pages = 0usize;
    let mut owned_leaf_pages = 0usize;
    for space in spaces.iter() {
        page_table_pages = page_table_pages.saturating_add(space.table_frames.len());
        owned_leaf_pages = owned_leaf_pages.saturating_add(space.leaf_frames.len());
    }
    VmmResourceStats {
        address_spaces: spaces.len(),
        page_table_pages,
        owned_leaf_pages,
    }
}

/// Free all frames owned by a non-current user address space.
pub fn free_address_space(pml4_frame: PhysFrame) {
    if pml4_frame == current_pml4() {
        return;
    }

    let space = {
        let mut spaces = ADDRESS_SPACES.lock();
        let Some(idx) = spaces.iter().position(|space| space.pml4 == pml4_frame) else {
            return;
        };
        spaces.swap_remove(idx)
    };

    for frame in space.leaf_frames {
        free_unmapped_frame(frame);
    }
    for frame in space.table_frames {
        free_unmapped_frame(frame);
    }
    free_unmapped_frame(space.pml4);
}

/// Load `pml4_frame` into CR3, switching to that address space.
///
/// # Safety
/// The PML4 must have valid kernel-half entries so that execution can continue
/// after the switch.  All currently-executing code must be reachable via the
/// new page table.
pub unsafe fn switch_to(pml4_frame: PhysFrame) {
    let (current, flags) = Cr3::read();
    if current != pml4_frame {
        Cr3::write(pml4_frame, flags);
    }
}

/// Switch back to the boot PML4 (used for kernel tasks with pml4=None).
///
/// # Safety
/// Same requirements as `switch_to`.
pub unsafe fn switch_to_boot() {
    let boot_phys = PhysAddr::new(*BOOT_PML4.lock());
    let frame = PhysFrame::containing_address(boot_phys);
    switch_to(frame);
}

/// Return the current PML4 physical frame.
pub fn current_pml4() -> PhysFrame {
    Cr3::read().0
}

pub fn user_range_accessible(ptr: u64, len: u64, writable: bool) -> bool {
    user_range_accessible_in(current_pml4(), ptr, len, writable)
}

pub fn user_range_accessible_in(pml4_frame: PhysFrame, ptr: u64, len: u64, writable: bool) -> bool {
    if ptr == 0 || len == 0 {
        return false;
    }
    let Some(last) = ptr.checked_add(len - 1) else {
        return false;
    };
    if last >= USER_TOP {
        return false;
    }
    let mut page_addr = ptr & !0xfffu64;
    loop {
        if !user_page_accessible_in(pml4_frame, VirtAddr::new(page_addr), writable) {
            return false;
        }
        if page_addr >= (last & !0xfffu64) {
            break;
        }
        page_addr = page_addr.saturating_add(4096);
    }
    true
}

pub fn valid_user_mmap_range(addr: u64, len_aligned: u64) -> bool {
    if addr & 0xfff != 0 || len_aligned == 0 || len_aligned & 0xfff != 0 {
        return false;
    }
    let Some(end) = addr.checked_add(len_aligned) else {
        return false;
    };
    addr >= USER_MMAP_BASE && end <= USER_MMAP_TOP
}

fn user_page_accessible_in(pml4_frame: PhysFrame, virt: VirtAddr, writable: bool) -> bool {
    let page: Page<Size4KiB> = Page::containing_address(virt);
    let pml4 = unsafe { table_at(pml4_frame.start_address()) };
    let l4 = &pml4[page.p4_index()];
    if !entry_allows(l4.flags(), writable) {
        return false;
    }
    let l3_table = unsafe { table_at(l4.addr()) };
    let l3 = &l3_table[page.p3_index()];
    if !entry_allows(l3.flags(), writable) {
        return false;
    }
    if l3.flags().contains(PageTableFlags::HUGE_PAGE) {
        return true;
    }
    let l2_table = unsafe { table_at(l3.addr()) };
    let l2 = &l2_table[page.p2_index()];
    if !entry_allows(l2.flags(), writable) {
        return false;
    }
    if l2.flags().contains(PageTableFlags::HUGE_PAGE) {
        return true;
    }
    let l1_table = unsafe { table_at(l2.addr()) };
    let l1 = &l1_table[page.p1_index()];
    entry_allows(l1.flags(), writable)
}

fn entry_allows(flags: PageTableFlags, writable: bool) -> bool {
    flags.contains(PageTableFlags::PRESENT)
        && flags.contains(PageTableFlags::USER_ACCESSIBLE)
        && (!writable || flags.contains(PageTableFlags::WRITABLE))
}

fn record_table_frames(pml4_frame: PhysFrame, mut frames: Vec<PhysFrame>) {
    if frames.is_empty() {
        return;
    }
    let mut spaces = ADDRESS_SPACES.lock();
    if let Some(space) = spaces.iter_mut().find(|space| space.pml4 == pml4_frame) {
        space.table_frames.append(&mut frames);
    }
}

fn record_leaf_frame(pml4_frame: PhysFrame, frame: PhysFrame) {
    let mut spaces = ADDRESS_SPACES.lock();
    if let Some(space) = spaces.iter_mut().find(|space| space.pml4 == pml4_frame) {
        space.leaf_frames.push(frame);
    }
}

fn clear_user_accessible_recursive(table: &mut PageTable, level: u8) {
    for entry in table.iter_mut() {
        if entry.is_unused() || !entry.flags().contains(PageTableFlags::PRESENT) {
            continue;
        }

        let flags = entry.flags();
        entry.set_flags(flags & !PageTableFlags::USER_ACCESSIBLE);
        if level <= 1 || flags.contains(PageTableFlags::HUGE_PAGE) {
            continue;
        }

        let next = unsafe { table_at(entry.addr()) };
        clear_user_accessible_recursive(next, level - 1);
    }
}

const fn pml4_index(addr: u64) -> usize {
    ((addr >> 39) & 0x1ff) as usize
}
