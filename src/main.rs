#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod abi;
mod accessibility;
mod acpi;
mod allocator;
mod app_lifecycle;
mod app_metadata;
mod apps;
mod ata;
mod boot_splash;
mod boot_watchdog;
mod branding;
mod browser_session;
mod browser_storage;
mod clipboard;
mod config_store;
mod coolfs;
mod crashdump;
mod deferred;
mod desktop_settings;
mod device_registry;
mod drivers;
mod elf;
mod entropy;
mod event_bus;
mod evented;
mod fat32;
mod font;
mod framebuffer;
mod fs_hardening;
mod fw_cfg;
mod gdt;
mod interrupts;
mod jobs;
mod keyboard;
mod klog;
mod memory;
mod mouse;
mod net;
mod notifications;
mod packages;
mod pci;
mod png;
mod process_model;
mod profiler;
mod recovery;
mod resource_limits;
mod rtc;
mod scheduler;
mod search_index;
mod security;
mod selftest;
mod services;
mod settings_state;
mod shortcuts;
mod slab;
mod syscall;
mod sysreport;
mod task_snapshot;
mod tls;
mod tls_roots;
mod tty;
mod usb;
mod userspace;
mod vfs;
mod vga_buffer;
mod virtio_net;
mod vmm;
mod wait_queue;
mod wm;
mod writeback;

use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;

/// Tell the bootloader to map all physical memory at a dynamic virtual
/// address so `boot_info.physical_memory_offset` is valid.
static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut cfg = BootloaderConfig::new_default();
    cfg.mappings.physical_memory = Some(Mapping::Dynamic);
    cfg.kernel_stack_size = 256 * 1024;
    cfg
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // ── Framebuffer ───────────────────────────────────────────────────────────
    // Grab the bootloader-provided framebuffer before any other init so that
    // `println!` (used by the panic handler) works as early as possible.
    if let Some(fb) = boot_info.framebuffer.as_mut() {
        let info = fb.info();
        let base = fb.buffer_mut().as_mut_ptr() as u64;
        let fmt = match info.pixel_format {
            bootloader_api::info::PixelFormat::Rgb => framebuffer::PixFmt::Rgb,
            _ => framebuffer::PixFmt::Bgr,
        };
        framebuffer::init(
            base,
            info.width,
            info.height,
            info.stride,
            info.bytes_per_pixel,
            fmt,
        );
        crate::vga_buffer::set_framebuffer_output(false);
        boot_splash::show("starting kernel", 0, boot_splash::BOOT_PROGRESS_TOTAL);
        println!(
            "FB {}x{} stride={} bpp={} base={:#x}",
            info.width, info.height, info.stride, info.bytes_per_pixel, base
        );
    } else {
        // No framebuffer — nothing will render but at least we don't crash silently.
        panic!("bootloader did not provide a framebuffer");
    }

    // ── Core kernel services ──────────────────────────────────────────────────
    gdt::init();
    boot_splash::show(
        "loading descriptor tables",
        1,
        boot_splash::BOOT_PROGRESS_TOTAL,
    );

    interrupts::init_idt();
    boot_splash::show(
        "registering interrupts",
        2,
        boot_splash::BOOT_PROGRESS_TOTAL,
    );

    unsafe { interrupts::PICS.lock().initialize() };
    interrupts::init_pit(interrupts::TIMER_HZ);
    boot_splash::show(
        "starting interrupt controller",
        3,
        boot_splash::BOOT_PROGRESS_TOTAL,
    );

    interrupts::mask_unused_irqs();
    syscall::init();
    x86_64::instructions::interrupts::enable();
    boot_splash::show("enabling syscalls", 4, boot_splash::BOOT_PROGRESS_TOTAL);

    let phys_mem_offset = x86_64::VirtAddr::new(
        boot_info
            .physical_memory_offset
            .into_option()
            .expect("physical memory offset not provided by bootloader"),
    );
    let mut mapper = unsafe { memory::init(phys_mem_offset) };

    // Convert the bootloader's MemoryRegions to a plain &'static slice.
    let regions: &'static [bootloader_api::info::MemoryRegion] =
        unsafe { core::mem::transmute(boot_info.memory_regions.as_ref()) };
    boot_splash::show("reading memory map", 5, boot_splash::BOOT_PROGRESS_TOTAL);

    // We need two separate frame allocators: one consumed by heap init, one kept
    // for the VMM.  The BootInfoFrameAllocator is cheap to reconstruct from the
    // same regions slice; each tracks its own `next` index independently.
    let mut heap_frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(regions) };
    boot_splash::show("reserving heap pages", 6, boot_splash::BOOT_PROGRESS_TOTAL);

    allocator::init_heap(&mut mapper, &mut heap_frame_allocator).expect("heap init failed");
    boot_splash::show("allocating heap", 7, boot_splash::BOOT_PROGRESS_TOTAL);
    klog::init();
    profiler::record_boot_stage("allocating heap", 7);
    boot_splash::show("mounting filesystems", 8, boot_splash::BOOT_PROGRESS_TOTAL);
    fs_hardening::init();
    event_bus::emit("boot", "heap", "kernel heap online");
    security::init();
    settings_state::init();
    if settings_state::snapshot().storage_fsck_on_boot {
        for line in fs_hardening::repair() {
            klog::log_owned(alloc::format!("fsck-on-boot: {}", line));
        }
    }
    app_lifecycle::init();
    packages::init();
    accessibility::load_from_disk();
    device_registry::refresh_pci();
    drivers::init();
    acpi::init(
        boot_info.rsdp_addr.as_ref().copied(),
        phys_mem_offset.as_u64(),
    );
    services::init();
    deferred::enqueue(deferred::DeferredWork::RefreshSearchIndex);
    deferred::enqueue(deferred::DeferredWork::FlushWriteback);

    // Build a fresh allocator starting after the frames consumed by the heap
    // (the heap allocator's `next` counter tells us how many frames it used).
    let vmm_frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init_from(regions, heap_frame_allocator.next()) };
    boot_splash::show("preparing page tables", 9, boot_splash::BOOT_PROGRESS_TOTAL);

    // Initialise the VMM with the physical-memory offset and the remaining
    // frame supply.  From here on, all page-table work goes through vmm::.
    vmm::init(phys_mem_offset, vmm_frame_allocator);
    vmm::harden_boot_mappings();
    font::load_from_disk();
    boot_splash::show(
        "mapping virtual memory",
        10,
        boot_splash::BOOT_PROGRESS_TOTAL,
    );

    boot_splash::show(
        "isolating kernel mappings",
        11,
        boot_splash::BOOT_PROGRESS_TOTAL,
    );

    // DMA-backed hardware probing needs heap/VMM/interrupts, but it should not
    // be gated on the scheduler or first desktop frame. Running it here keeps
    // headless device smoke tests deterministic as shell setup grows.
    net::init();
    usb::init();

    mouse::init_cursor();
    boot_splash::show(
        "preparing desktop shell",
        12,
        boot_splash::BOOT_PROGRESS_TOTAL,
    );
    wm::prepare();

    // ── Scheduler ─────────────────────────────────────────────────────────────
    boot_splash::show("starting scheduler", 15, boot_splash::BOOT_PROGRESS_TOTAL);
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut sched = scheduler::SCHEDULER.lock();
        sched.add_idle();
    });
    selftest::run_boot_tests();
    fs_test_once();
    boot_splash::show(
        "staging filesystem checks",
        16,
        boot_splash::BOOT_PROGRESS_TOTAL,
    );

    // Spawn both boot sentinels before either can run. The ELF loader touches
    // VFS and VMM locks; letting the first sentinel exit while the boot task is
    // still loading the second can deadlock on those locks.
    x86_64::instructions::interrupts::without_interrupts(|| {
        userspace::spawn_user_process(1);
        userspace::spawn_user_process(2);
    });
    boot_splash::show("launching userspace", 17, boot_splash::BOOT_PROGRESS_TOTAL);

    // ── Desktop ───────────────────────────────────────────────────────────────
    shortcuts::load_from_disk();
    wm::init();
    boot_splash::show("drawing desktop", 23, boot_splash::BOOT_PROGRESS_TOTAL);
    wm::compose_if_needed();
    println!(
        "[ui] ready pinned={}",
        app_lifecycle::pinned_order_summary()
    );
    println!("[boot] login ready");
    println!("[boot] desktop ready");
    profiler::record_boot_stage("desktop ready", boot_splash::BOOT_PROGRESS_TOTAL);
    boot_watchdog::complete();
    let smoke_commands = fw_cfg::smoke_commands();
    if !smoke_commands.is_empty() {
        apps::terminal::set_debug_mirror(true);
        wm::queue_startup_command_immediate("login root cool");
    }
    let immediate_smoke = smoke_commands.iter().all(|command| {
        matches!(
            command.split_whitespace().next(),
            Some("vfs" | "path" | "df" | "fsck")
        )
    });
    for command in smoke_commands {
        println!("[smoke] command {}", command);
        if immediate_smoke {
            wm::queue_startup_command_immediate(&command);
        } else {
            wm::queue_startup_command(&command);
        }
    }

    loop {
        // Do NOT disable interrupts here — the WM mutex inside compose()
        // provides the only exclusion needed.  Holding interrupts off for
        // an entire frame (≈2.8 M MMIO writes at 1280×720×3 bpp) would
        // block mouse and keyboard for tens of milliseconds per frame.
        usb::poll();
        wm::compose_if_needed();
        services::supervise();
        deferred::drain_budget(1);
        net::poll();
        wm::compose_if_needed();
        x86_64::instructions::hlt();
    }
}

/// One-shot boot check: reads /bin/hello.txt from the CoolFS root and prints it.
fn fs_test_once() {
    println!("[fs] check started");
    match vfs::vfs_read_file("/bin/hello.txt") {
        Some(bytes) => {
            print!("[fs] /bin/hello.txt: ");
            for b in &bytes {
                vga_buffer::_print(core::format_args!("{}", *b as char));
            }
        }
        None => println!("[fs] /bin/hello.txt: NOT FOUND"),
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    klog::log("kernel panic");
    crashdump::record_panic(info);
    klog::dump_to_console();
    crate::vga_buffer::reset_cursor();
    crate::vga_buffer::set_framebuffer_output(true);
    println!("{}", info);
    loop {}
}
